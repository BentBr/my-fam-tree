import { expect, test } from '../fixtures/console.fixture'
import { rewriteEmailLink } from '../fixtures/email-links.fixture'
import { clearMailpit, waitForEmail } from '../fixtures/mailpit.fixture'
import { signIn, createFamily } from '../page-objects/session'

test('contact role + visibility gates', async ({ browser }) => {
    const stamp = Date.now()
    // ----- Owner context: signs in, creates family, sets up two persons. -----
    const ownerCtx = await browser.newContext()
    const owner = await ownerCtx.newPage()
    const ownerEmail = `owner-contacts-${stamp}@example.com`
    await signIn(owner, ownerEmail)
    const familyId = await createFamily(owner, `ContactsFam-${stamp}`)

    // ----- Guest context: signs in to provision a user row. -----
    const guestCtx = await browser.newContext()
    const guest = await guestCtx.newPage()
    const guestEmail = `guest-contacts-${stamp}@example.com`
    await signIn(guest, guestEmail)

    // ----- Owner invites guest, guest accepts. -----
    await clearMailpit()
    const inviteRes = await owner.request.post(`/api/v1/families/${familyId}/invites`, {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { email: guestEmail, role: 'user' },
    })
    expect(inviteRes.ok()).toBeTruthy()
    const inviteMail = await waitForEmail((s) => /Join the .+ family on my-fam-tree|Einladung zur Familie/.test(s), {
        recipient: guestEmail,
    })
    const inviteMatch = inviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (inviteMatch === null) throw new Error('invite link not in email')
    const inviteLink = inviteMatch[0]
    if (inviteLink === undefined) throw new Error('invite link empty')
    await guest.goto(rewriteEmailLink(inviteLink))
    await expect(guest).toHaveURL(/\/(tree|invite\/accept)/)

    // ----- Owner creates two persons: one unlinked, one linked to the guest. -----
    // The link to the guest must go through a consent-safe path — the
    // BE's `check_link_consent` gate rejects an admin posting an
    // arbitrary `linked_user_id` in the CREATE body (that was the old
    // silent-bind hole). Two consent-safe paths exist; we use the
    // person-targeted invite here because the guest is a separate
    // user-account (the `claim` endpoint is self-link only). The BE's
    // invite-accept handler atomically sets `linked_user_id` on the
    // bound person when the recipient clicks the link.

    // Owner-only person.
    const ownerPersonRes = await owner.request.post('/api/v1/persons', {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { given_name: 'OwnerPerson' },
    })
    expect(ownerPersonRes.ok()).toBeTruthy()
    const ownerPersonId = ((await ownerPersonRes.json()) as { data: { id: string } }).data.id

    // Guest-linked person — created unlinked first, then bound via a
    // person-targeted invite that the guest accepts.
    const guestPersonRes = await owner.request.post('/api/v1/persons', {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { given_name: 'GuestPerson' },
    })
    expect(guestPersonRes.ok()).toBeTruthy()
    const guestPersonId = ((await guestPersonRes.json()) as { data: { id: string } }).data.id

    await clearMailpit()
    const personInviteRes = await owner.request.post(`/api/v1/families/${familyId}/invites`, {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { email: guestEmail, role: 'user', person_id: guestPersonId },
    })
    expect(personInviteRes.ok()).toBeTruthy()
    const personInviteMail = await waitForEmail(
        (s) => /Join the .+ family on my-fam-tree|Einladung zur Familie/.test(s),
        { recipient: guestEmail },
    )
    const personInviteMatch = personInviteMail.text.match(/https?:\/\/\S+\/invite\/accept\?token=\S+/)
    if (personInviteMatch === null) throw new Error('person-targeted invite link not in email')
    const personInviteLink = personInviteMatch[0]
    if (personInviteLink === undefined) throw new Error('person-targeted invite link empty')
    await guest.goto(rewriteEmailLink(personInviteLink))
    // InviteAccept redirects to /tree on a successful link (existing-user
    // path); we wait for that as proof the link landed.
    await expect(guest).toHaveURL(/\/(tree|invite\/accept)/, { timeout: 15_000 })

    // ----- Owner seeds three contacts on the owner-person:
    //       one family, one admins_only, one address.
    const seedContacts = [
        { kind: 'email', value: { email: 'owner@example.com' }, visibility: 'family' },
        { kind: 'email', label: 'Private', value: { email: 'owner-private@example.com' }, visibility: 'admins_only' },
        { kind: 'phone', value: { number: '+49 30 5550100' }, visibility: 'family' },
    ]
    for (const c of seedContacts) {
        const r = await owner.request.post(`/api/v1/persons/${ownerPersonId}/contacts`, {
            headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
            data: c,
        })
        expect(r.ok()).toBeTruthy()
    }

    // ----- Guest opens their own person — they may add a contact. -----
    await guest.goto('/tree')
    // Sibling tests in this directory use `dispatchEvent('click')` to
    // bypass Vuetify's navigation-drawer scrim that otherwise intercepts
    // pointer events on initial render.
    await expect(guest.getByTestId(`tree-node-${guestPersonId}`)).toBeVisible()
    await guest.getByTestId(`tree-node-${guestPersonId}`).dispatchEvent('click')

    // Add a contact on their own person via the UI (Add button visible).
    await expect(guest.getByTestId('contact-add')).toBeVisible()
    await guest.getByTestId('contact-add').click()
    await guest.getByTestId('contact-kind').selectOption('email')
    await guest.getByTestId('contact-value').fill('guest@example.com')
    await guest.getByTestId('contact-submit').click()
    await expect(guest.getByText('guest@example.com')).toBeVisible()

    // ----- Guest opens the OWNER's person.
    await guest.getByTestId(`tree-node-${ownerPersonId}`).dispatchEvent('click')

    // Add button is hidden (no edit rights).
    await expect(guest.getByTestId('contacts-section')).toBeVisible()
    await expect(guest.getByTestId('contact-add')).toHaveCount(0)
    // admins_only contact is hidden — only the family-visible rows render.
    await expect(guest.getByText('owner@example.com')).toBeVisible()
    await expect(guest.getByText('owner-private@example.com')).toHaveCount(0)

    // ----- Guest attempts a direct API edit on the owner's contact → 403. -----
    const contacts = await guest.request.get(`/api/v1/persons/${ownerPersonId}/contacts`, {
        headers: { 'X-Family-Id': familyId },
    })
    expect(contacts.ok()).toBeTruthy()
    const { data } = (await contacts.json()) as { data: { contacts: Array<{ id: string }> } }
    const someId = data.contacts[0]?.id
    if (someId === undefined) throw new Error('expected at least one contact for owner person')
    const patchRes = await guest.request.patch(`/api/v1/contacts/${someId}`, {
        headers: { 'X-Family-Id': familyId, 'content-type': 'application/json' },
        data: { kind: 'email', value: { email: 'evil@example.com' }, visibility: 'family' },
    })
    expect(patchRes.status()).toBe(403)
    const patchBody = (await patchRes.json()) as { code: string }
    expect(patchBody.code).toBe('contact_not_editable')

    await ownerCtx.close()
    await guestCtx.close()
})
