# Changelog

## [0.1.14](https://github.com/BentBr/my-fam-tree/compare/core-v0.1.13...core-v0.1.14) (2026-05-31)


### Bug Fixes

* **fe:** cache-bust user-visible brand imagery via Vite content hashes ([dadc198](https://github.com/BentBr/my-fam-tree/commit/dadc198cb2d5f61a13c7d5c4e184846f0c547d70))
* **fe:** only split open partners around anchor when there are NO ended partners ([c780e97](https://github.com/BentBr/my-fam-tree/commit/c780e97e40baeca36182c9d894832ed4a875c7ea))
* **fe:** parent-edge-aware root barycenter (Herta/Anneliese crossing case) ([a9c5c36](https://github.com/BentBr/my-fam-tree/commit/a9c5c36f0c396e87165922d23574a1d0ef4e7ea9))
* **fe:** three layout bugs (sibling-by-age, anchor-in-middle, multi-row crossing) ([d8adbcc](https://github.com/BentBr/my-fam-tree/commit/d8adbccefcc08ba3d921ebf51253141780ffd48b))

## [0.1.13](https://github.com/BentBr/my-fam-tree/compare/core-v0.1.12...core-v0.1.13) (2026-05-31)


### Bug Fixes

* **auth:** invite-accept mints a refresh cookie + drop dev/test debug info for CI disk budget ([c00a533](https://github.com/BentBr/my-fam-tree/commit/c00a533476e3a07b23baa0ef46e4228795e56df3))
* **ci:** rustfmt import order + skip authRefresh for /invites/accept + e2e adjustments ([be6824b](https://github.com/BentBr/my-fam-tree/commit/be6824bb9050e984f858f4e940bb05d66100a8ae))

## [0.1.12](https://github.com/BentBr/my-fam-tree/compare/core-v0.1.11...core-v0.1.12) (2026-05-31)


### Features

* **admin:** /admin/family overview page with rename + stat tiles + latest 3 ([81f5cdb](https://github.com/BentBr/my-fam-tree/commit/81f5cdb5a0a981927db0dbb130a27b18672020ea))


### Bug Fixes

* consent bypass on PATCH /persons, retry body re-use, e2e cold-start flakes ([924eff2](https://github.com/BentBr/my-fam-tree/commit/924eff2c0df714222a88b2d07a65631dcc8bc81e))
* **e2e:** flush redis between tests to drain rate-limit buckets ([0f2b506](https://github.com/BentBr/my-fam-tree/commit/0f2b5067e19d646b1116770b52dca99f176b04c5))
* linked-person fallback in admin/audit, sub-ms health timings, latest-person index ([9ec3488](https://github.com/BentBr/my-fam-tree/commit/9ec348889af17fb188bc5b7e79ec17f52bd41b04))
* SQLX_OFFLINE prepare cache + shared form primitives + health network chip + AppBar tree link ([fd2a43d](https://github.com/BentBr/my-fam-tree/commit/fd2a43d65c1760cfe7631349f563b3908e030ae2))
* **test:** align link_consent unit tests with the tightened 'current.is_none()' semantics ([d5acaf9](https://github.com/BentBr/my-fam-tree/commit/d5acaf95ab2b33473049aa81b3ded25a90104aa9))

## [0.1.11](https://github.com/BentBr/my-fam-tree/compare/core-v0.1.10...core-v0.1.11) (2026-05-31)


### Features

* **brand:** swap neutral tree-example WebPs for light/dark theme variants ([52fd9d8](https://github.com/BentBr/my-fam-tree/commit/52fd9d8c75291bc6a50009dff7929ac2166e4b1a))
* **health:** worker liveness probe — API `worker_ok` chip + worker `/health` ([96a3112](https://github.com/BentBr/my-fam-tree/commit/96a31128f5a7b20a24f2cefc3a7e0390656afeae))
* **mobile:** move add-person CTA into toolbar, collapse family switcher to icon ([c65ac54](https://github.com/BentBr/my-fam-tree/commit/c65ac54b9d7c61ce459f109fe9bb0a8d2d7baadc))
* **seed:** Krause subtree + layout-edge-case tests; fix avatar e2e PNG ([e5a1a9c](https://github.com/BentBr/my-fam-tree/commit/e5a1a9c405f24d17ca3ae465af8bc009deb08d39))
* **tree:** marriage rings + ended-state grey-out + adjacency-aware line + contacts contrast ([1971afb](https://github.com/BentBr/my-fam-tree/commit/1971afbd9039712a49af08530253874c6c6a0e9c))
* **tree:** two-pass barycenter layout — kills avoidable parent-edge crossings ([0283e2f](https://github.com/BentBr/my-fam-tree/commit/0283e2f1d8ecdf50233fd450648a28ba8f000826))


### Bug Fixes

* **e2e:** replace test PNG with valid-CRC bytes ([f13569e](https://github.com/BentBr/my-fam-tree/commit/f13569e8e9d3913524ec38d6f6a13e60019206bc))
* **fe:** mobile add-person FAB + invite padding + GH-source footer link + theme-resolved home screenshot ([962ba1c](https://github.com/BentBr/my-fam-tree/commit/962ba1c0b198eee4af6a4a42a6540da4509bff0f))
* **fe:** reorder middleware so authRefresh sees 401s before errorTranslator throws ([b2ba37e](https://github.com/BentBr/my-fam-tree/commit/b2ba37e118ea2a777cef8fded387412efab420b4))
* **fe:** suppress iOS keyboard on gender combobox via inputmode=none ([7994a23](https://github.com/BentBr/my-fam-tree/commit/7994a2372f75c9243266bf1d3031696c47984021))
* **router:** /account + /health don't require an active family ([ecaa939](https://github.com/BentBr/my-fam-tree/commit/ecaa9391198f306af63db84b236f69094c882b96))
* **tree:** keep mobile FAB floating on top + hide fit-to-view on mobile ([ea3b349](https://github.com/BentBr/my-fam-tree/commit/ea3b349ef6b243bff9d43f68b1388c9fbfb0a922))
* **tree:** replace v-btn/v-fab mobile FAB with hand-rolled button ([5898bfe](https://github.com/BentBr/my-fam-tree/commit/5898bfe018bba4e6dbb544a144a76cc2c0d36861))
* **tree:** teleport mobile add-person FAB to body so it escapes ancestor containing-block traps ([7be6c21](https://github.com/BentBr/my-fam-tree/commit/7be6c21d40f5e8ebe6c2ab3edccbe79641bcf894))

## [0.1.10](https://github.com/BentBr/my-fam-tree/compare/core-v0.1.9...core-v0.1.10) (2026-05-31)


### Features

* **health:** add server_duration_ms timer alongside the DB latency ([48fbd76](https://github.com/BentBr/my-fam-tree/commit/48fbd7608dfcd6a1869d4549408898803f72301f))
* **tree:** allow deeper manual zoom-out without changing fit-to-view ([3ba51ed](https://github.com/BentBr/my-fam-tree/commit/3ba51edae37f350e7f04d061b11fdc12aa3f1aca))

## [0.1.9](https://github.com/BentBr/my-fam-tree/compare/core-v0.1.8...core-v0.1.9) (2026-05-31)


### Bug Fixes

* **api:** PATCH /persons/{id} actually clears birth_date / death_date on null ([640c372](https://github.com/BentBr/my-fam-tree/commit/640c372ce316e0a98a7d0dfaf367f7758d1b3eb5))
* **auth:** make /auth/logout public so the FE can drop stale cookies after session collapse ([d92a4f9](https://github.com/BentBr/my-fam-tree/commit/d92a4f987b79a18dddde942ecc397400a27cb5a3))
* **e2e:** use dispatchEvent('click') on tree-node to bypass v-navigation-drawer scrim ([9f3ccf7](https://github.com/BentBr/my-fam-tree/commit/9f3ccf71da53880221a45396a19e4f3afa6591f9))
* **fe:** try refresh on any 401, not only `auth_token_expired` ([39e6f18](https://github.com/BentBr/my-fam-tree/commit/39e6f18ce62e1b92a62c05fd49242ebe04180cdd))
* two follow-ups from the public-logout move ([5c962a5](https://github.com/BentBr/my-fam-tree/commit/5c962a5e7e94cf14c673b7ae0fb9a10e5000cec9))

## [0.1.8](https://github.com/BentBr/my-fam-tree/compare/core-v0.1.7...core-v0.1.8) (2026-05-30)


### Features

* **persons:** self-claim — admin/owner can link a person row to themselves in one click ([14c24a4](https://github.com/BentBr/my-fam-tree/commit/14c24a42bc129c9f4d20c7a8d61323b119351601))


### Bug Fixes

* **api:** close the consent hole on persons CREATE / PATCH ([bb23753](https://github.com/BentBr/my-fam-tree/commit/bb2375370f4fd2a954649ad2a191aa7534bb46a5))
* **api:** CORS — allow PUT method + decorate 401s from the auth middleware ([5809fe1](https://github.com/BentBr/my-fam-tree/commit/5809fe1260774334973b6dc513b85d59c7455b05))
* **api:** make memberships.insert idempotent — re-accepting an invite is a no-op ([6ef9e1c](https://github.com/BentBr/my-fam-tree/commit/6ef9e1c4d89e567a9565fa37dc590046af7bca54))

## [0.1.7](https://github.com/BentBr/my-fam-tree/compare/core-v0.1.6...core-v0.1.7) (2026-05-30)


### Bug Fixes

* **fe:** split mutation / navigation try-catch in token-consume views ([d2ba9ff](https://github.com/BentBr/my-fam-tree/commit/d2ba9ffac3612856c2c5b8e4c231430b4e46a6cb))

## [0.1.6](https://github.com/BentBr/my-fam-tree/compare/core-v0.1.5...core-v0.1.6) (2026-05-30)


### Features

* **fe:** public marketing site + sitemap/robots + nginx delivery ([f44425a](https://github.com/BentBr/my-fam-tree/commit/f44425a3fd3948f2b6d18511b52eaf76101d95bb))
* **fe:** Slothlike design system + unified AppBar chrome + brand assets ([d2ed3fa](https://github.com/BentBr/my-fam-tree/commit/d2ed3fab4ed8b5da6071f6dd642fe3a1e044821d))
* **fe:** unified sidebar, hover-lineage tree glow, public-page screenshot + privacy refresh ([4dae5cf](https://github.com/BentBr/my-fam-tree/commit/4dae5cfb19b4b2362ad90e9218e8e418e409cd39))


### Bug Fixes

* **fe:** keep the contact email out of the i18n message compiler ([540c60c](https://github.com/BentBr/my-fam-tree/commit/540c60c804fc9bcf371398df073e6ef105265c1e))
* **fe:** stabilize the post-rebrand e2e flakes ([3692d2d](https://github.com/BentBr/my-fam-tree/commit/3692d2d6454ef7f7ab1ee900d96516b31991c05c))

## [0.1.5](https://github.com/BentBr/my-fam-tree/compare/core-v0.1.4...core-v0.1.5) (2026-05-29)


### Features

* **api:** convert remaining email producers to the outbox ([54f3ea0](https://github.com/BentBr/my-fam-tree/commit/54f3ea00d60647749fb0bcc977bddc56f847af34))
* **api:** photo-upload foundation + new crate-config / crate-storage skills ([056d2a3](https://github.com/BentBr/my-fam-tree/commit/056d2a3369aa34b4dfec6571c48d9d94461cb7b0))
* **api:** POST/DELETE /persons/{id}/photo with image validation + presigned URLs ([73c16f8](https://github.com/BentBr/my-fam-tree/commit/73c16f8401466756d9c48fdbdac92b103121f806))
* **api:** POST/DELETE /users/me/avatar mirroring person photos ([a3ebf21](https://github.com/BentBr/my-fam-tree/commit/a3ebf219545c69a58c35fccd540b6c9360fc189b))
* **config,storage:** centralised config crate + Task 5 storage scaffold ([922420d](https://github.com/BentBr/my-fam-tree/commit/922420dc75629b240d1c36ee68dd87892cc93960))
* **domain,persistence:** plumb photo_key through Person aggregate + repo ([dd4cf66](https://github.com/BentBr/my-fam-tree/commit/dd4cf66870622ca13b3c167bebd666f5399c9c50))
* **fe:** PersonDetail photo upload + DefaultAvatar fallback ([a22d2dd](https://github.com/BentBr/my-fam-tree/commit/a22d2dd91f6e5986d86bbbb95e50d1c710dd7a60))
* **fe:** soft-confirm on duplicate-owned family + Task 23 edge-case e2e ([30ef22a](https://github.com/BentBr/my-fam-tree/commit/30ef22ac8402d82b434bb10b97770a198e28cd41))
* **fe:** user avatar upload in account view + nav menu ([b73269a](https://github.com/BentBr/my-fam-tree/commit/b73269ad470f91dadbd3ee9374f990f12a4984d5))
* **infra:** MinIO two-service setup via nginx proxy on port 80 ([2e75c7b](https://github.com/BentBr/my-fam-tree/commit/2e75c7b9d9b470a791e403e941e64a428ca529a1))
* **photo:** tree-node photos + hero sidebar + user-avatar propagation ([784553c](https://github.com/BentBr/my-fam-tree/commit/784553c9e088600f08378a20df117ff37a6a0585))
* **worker,api:** durable email outbox + magic-link goes through it ([1e60501](https://github.com/BentBr/my-fam-tree/commit/1e6050177fc2a0d203d91603d0fc3df2b91a3e5d))
* **worker:** periodic janitor — sweep expired auth/invite/transfer rows ([0fdec92](https://github.com/BentBr/my-fam-tree/commit/0fdec9246829d03d465d7e7c6d970d0e95674374))


### Bug Fixes

* **api:** restore bytes dep dropped by the rebrand sed ([c1295d4](https://github.com/BentBr/my-fam-tree/commit/c1295d461e4682bf4505c4a28d213fe9546c3db8))
* **ci:** bump magic-link caps and pick a probe date that won't freeze the worker ([fa9e667](https://github.com/BentBr/my-fam-tree/commit/fa9e667c0fd097ae9af737cc9f3868b3bca6f50d))
* **ci:** regenerate openapi spec + repair stale unit-test expectations ([886e5e2](https://github.com/BentBr/my-fam-tree/commit/886e5e2a79f39c10c89cf52531fa4398f29b455e))
* **ci:** release-please json updater shape + test outbox SyncOutbox double ([4ccb4cb](https://github.com/BentBr/my-fam-tree/commit/4ccb4cb0cc9d2242060bba4c3af55760f55f8187))
* **ci:** worker needs janitor + outbox env vars or it dies at startup ([73bcc5a](https://github.com/BentBr/my-fam-tree/commit/73bcc5ad9ccb669088b14e516f5d3f22bdf55f9f))
* **config:** drop deny_unknown_fields so figment Env::raw ignores PATH/HOME/_ ([263128d](https://github.com/BentBr/my-fam-tree/commit/263128de84d9018d156b6efe3bf2b4c024bad29b))
* **docker:** match runtime image to builder so binaries actually run ([5a33594](https://github.com/BentBr/my-fam-tree/commit/5a33594b4e948527f8e28925600d51b8f5824bab))
* **e2e:** lift compose rate caps + route playwright at the worker compose alias ([225d21a](https://github.com/BentBr/my-fam-tree/commit/225d21acf827fb064289e93f5dea8732b9322d5c))
* **fe:** dedupe identical toasts so a 401 burst surfaces once ([ac38003](https://github.com/BentBr/my-fam-tree/commit/ac3800382f1ede6a50dbf97c1647cb50cf62da52))
* **fe:** gate useMe on authenticated state so the nav avatar doesn't 401 ([e1fd6cc](https://github.com/BentBr/my-fam-tree/commit/e1fd6cc828315d4e48891203489007b90340bad4))
* **fe:** gate useMyFamilies on authenticated session — kill spurious 401 toast ([0b0262f](https://github.com/BentBr/my-fam-tree/commit/0b0262f68a89fcc8ee5fa3671a016bc471ac7ba6))
* **fe:** mobile gaps — admin sidebar collapses, tree heading shows family name ([5112556](https://github.com/BentBr/my-fam-tree/commit/511255670c90d41321dca55afe0dc57f245b443c))
* **fe:** user-menu button exposes the email via aria-label ([c4ea94d](https://github.com/BentBr/my-fam-tree/commit/c4ea94df7f4e14c6707b2d77e0bfc62589891734))
* **infra:** minio-api on port 80 so the SAME hostname works inside + outside ([50b3e7b](https://github.com/BentBr/my-fam-tree/commit/50b3e7b7a401949ee61d4016d5d712c98c61b016))
* **migrations:** make 0010/0011 idempotent so replays over partial state work ([5e1bc73](https://github.com/BentBr/my-fam-tree/commit/5e1bc73bf7573942d2b3948b88b9d66a2a6227b3))
* **owner-transfer:** atomic completion in a single transaction ([d0d97ac](https://github.com/BentBr/my-fam-tree/commit/d0d97acd115625293916ab94efd0ac2e53abc478))
* **security:** centralized DB-level role check for authz-sensitive writes ([4f352f6](https://github.com/BentBr/my-fam-tree/commit/4f352f60e720a096771b7a944dab393e5bec7fb0))
* **security:** close audit High + 5 Medium findings ([cf0c9b2](https://github.com/BentBr/my-fam-tree/commit/cf0c9b246e9bb54a720aa9e9b617d5de20d3ed95))
* **security:** close the 4 audit Low findings ([964c299](https://github.com/BentBr/my-fam-tree/commit/964c2990055ec040f58a5ef4cf13b4df07ba3a16))
* **security:** per-IP rate cap on token-validation endpoints ([d984474](https://github.com/BentBr/my-fam-tree/commit/d984474bbafdadeda1d0c6e39add560b3b9f4e8e))
* **security:** tighten production config validation ([398fe14](https://github.com/BentBr/my-fam-tree/commit/398fe146fc96c563a3398fd908154af3325f26c1))
* **sqlx:** include test-target queries in offline cache ([91fb7e4](https://github.com/BentBr/my-fam-tree/commit/91fb7e456d042a4d7525b5ef1f27de3ec1e90c6d))
* **storage,api:** async presigned_get + deny_unknown_fields on body DTOs ([a420ccd](https://github.com/BentBr/my-fam-tree/commit/a420ccd1fb9470b68f350f73e38a8f8a7327d6b8))
* **storage:** wire a real async sleep_impl into the S3 client ([07f4d60](https://github.com/BentBr/my-fam-tree/commit/07f4d604831f69840dd644bb052054713d470074))
* **worker tests:** future-shift FixedClock so claim_next_due isn't racy ([4e14acc](https://github.com/BentBr/my-fam-tree/commit/4e14acc27e410470cbd4b1283220e92863570567))
* **worker:** OffsetClock keeps the outbox draining under test-fixtures ([db3cf48](https://github.com/BentBr/my-fam-tree/commit/db3cf48b05aab944d725d10ab62eb277747a8f22))

## [0.1.4](https://github.com/BentBr/my-family/compare/core-v0.1.3...core-v0.1.4) (2026-05-28)


### Features

* **api:** expose created_at on GET /families/me (Phase 5 Task 23) ([1343053](https://github.com/BentBr/my-family/commit/13430537e5f7bf9a4a06c794e1520cfe46c48fd5))
* **fe:** disambiguate same-named families in picker + switcher; fix picker nav ([240ee0c](https://github.com/BentBr/my-family/commit/240ee0c4497e0896415ba7f4cc6e56ceabbd8a8e))
* **fe:** mobile-responsive nav drawer + tree heading (Phase 5 Task 20) ([a765463](https://github.com/BentBr/my-family/commit/a765463e2e88ef566a8a1d0380061ef572a247bd))


### Bug Fixes

* **release:** sync fe/openapi.json + Cargo.lock to 0.1.3 ([3f3cf76](https://github.com/BentBr/my-family/commit/3f3cf7665b21ab6554379be72ade6e37b7ebbec3))
* **test:** green up main — fmt, role-gate code string, invite-accept reload race ([3c9c667](https://github.com/BentBr/my-family/commit/3c9c66780c78a86c1a5c1e4fe8b627cf0bdcf1c9))

## [0.1.3](https://github.com/BentBr/my-family/compare/core-v0.1.2...core-v0.1.3) (2026-05-27)


### Features

* **api:** /health probes DB reachability + latency, always 200 ([e23aeda](https://github.com/BentBr/my-family/commit/e23aeda17399bd1c77b5ef0e2a3adb2d8344c7c2))
* **api:** GET/PUT /reminder-preferences (per-user mail settings) ([9d75f9b](https://github.com/BentBr/my-family/commit/9d75f9be2dbdf665ccb2b9d6d01cffa2dcaf431d))
* **cache:** redis reminder digest queue (non-blocking RPOP + push) ([d9259c6](https://github.com/BentBr/my-family/commit/d9259c62c5f700faeffbcd9316b11d9f159e5ef6))
* **email:** reminder digest template (en + de) listing N events ([a59fd70](https://github.com/BentBr/my-family/commit/a59fd704e3f099ad1dd1174100a892609dd7b0dd))
* **fe:** demote Health to a drawer footnote + show DB latency ([50c7baa](https://github.com/BentBr/my-family/commit/50c7baa504124136d9df696a2b0a8cc73cca1fce))
* **fe:** reminder-preferences panel in account settings ([e6b56f9](https://github.com/BentBr/my-family/commit/e6b56f9ffc5a49456a75e1ed8c28b4130804b57c))
* **persistence:** reminder preferences + digest log (migration 0009 + repos) ([a5f666c](https://github.com/BentBr/my-family/commit/a5f666cb0db39741af2b1b51c337aa9d299fc661))
* **rdt:** trigger-clock clears the day's digest so re-runs resend ([da53c3d](https://github.com/BentBr/my-family/commit/da53c3d1de3733bcfd94a12bde6e234995d95814))
* **reminder-worker:** clock-advance dinghy alias + date input + rdt trigger-clock ([06490df](https://github.com/BentBr/my-family/commit/06490dfd3ddbf8d08522b7158056b43fb24697b7))
* **reminder-worker:** leader-locked digest scheduler + dispatcher ([868e930](https://github.com/BentBr/my-family/commit/868e9309dd9e9be07543cd87145c0f3d80c71ea3))


### Bug Fixes

* **api:** default API_ENABLE_DOCS to false (prod-safe docs gate) ([0fdc0c3](https://github.com/BentBr/my-family/commit/0fdc0c30db7dced239065b316a72feb753cc2cdc))

## [0.1.2](https://github.com/BentBr/my-family/compare/core-v0.1.1...core-v0.1.2) (2026-05-26)


### Features

* **api:** 0005 migration drops flat contact cols, adds contacts + audit_log ([0f111e0](https://github.com/BentBr/my-family/commit/0f111e04c3887b94f165b9dd6345aa620e1c75cd))
* **api:** 0006 migration adds invite.person_id ([6dfe10e](https://github.com/BentBr/my-family/commit/6dfe10e0fab502e9bff58538e350c6897a226156))
* **api:** 0007 migration adds family_owner_transfers ([34e7b05](https://github.com/BentBr/my-family/commit/34e7b05be25dff9c5a191c1d010895443bd5049c))
* **api:** 0008 person_favourites join + repo trait + Pg impl ([a4e82ab](https://github.com/BentBr/my-family/commit/a4e82ab8a43c226afc6a1a3ecf6c9994b2013cc3))
* **api:** accept writes verify audit + sets linked_user_id ([4debc52](https://github.com/BentBr/my-family/commit/4debc52425b71b7f0695f6328b648616e0fb4c46))
* **api:** add MembershipNotFound error variant ([23a0f81](https://github.com/BentBr/my-family/commit/23a0f8190009c9111843bc4cc2fd0a7fd33fe2ea))
* **api:** ApiError::InviteDuplicate + 409 mapping + i18n ([99ec4af](https://github.com/BentBr/my-family/commit/99ec4af59ce817863e9da14168cfd1749dc1c9ed))
* **api:** contact CRUD endpoints with role + visibility gates + audit_log ([43175a7](https://github.com/BentBr/my-family/commit/43175a74b5cace050b2eeb1cd10f4e35d53f9cdf))
* **api:** families::invite carries person_id + duplicate guard ([1a02b35](https://github.com/BentBr/my-family/commit/1a02b35c56d158be45424f10126be9333c9bc3c6))
* **api:** favourite toggle route, tree + person + upcoming integration ([fa6a47f](https://github.com/BentBr/my-family/commit/fa6a47f1e446d802bbd852c0e6a64b7547e51d64))
* **api:** GET /families/{id}/audit with filters + pagination ([76a7ced](https://github.com/BentBr/my-family/commit/76a7ced9fbe1b2afb09ffed63bc967e2943b9da3))
* **api:** GET + DELETE families/{id}/invites endpoints ([1e29c9b](https://github.com/BentBr/my-family/commit/1e29c9bc66c643a89405a609696515135d46cfe0))
* **api:** GET/PATCH/DELETE family members with role matrix ([72ba2b5](https://github.com/BentBr/my-family/commit/72ba2b578e57f32cbf34fa811611e10361fbcc12))
* **api:** mount PgOwnerTransferRepo in AppState ([40fa164](https://github.com/BentBr/my-family/commit/40fa164d93156e5744e8cc14761fa99258a6cfa5))
* **api:** owner transfer routes (begin/confirm/cancel/status) ([63b7873](https://github.com/BentBr/my-family/commit/63b7873b1bd4297025265582baee3c719e1b5c99))
* **api:** OwnerTransferPending ApiError variant (409 conflict) ([b7cc2ef](https://github.com/BentBr/my-family/commit/b7cc2efbce19a1680a3692d61f99f29a58a5ce5e))
* **domain:** AuditLogRepo::list_filtered + AuditRow/Filter types ([7612b6d](https://github.com/BentBr/my-family/commit/7612b6de12ee1965e8511100e7735c9333ec186e))
* **domain:** FamilyInvite carries optional person_id ([405fb5b](https://github.com/BentBr/my-family/commit/405fb5b965c16eb394e37694efae87670eee782b))
* **domain:** MemberWithUser + FamilyMembershipRepo::list_with_users ([36624e4](https://github.com/BentBr/my-family/commit/36624e49d1e3b82619c4e836a005afd85c6d645f))
* **domain:** OwnerTransferRepo trait + types ([6304e19](https://github.com/BentBr/my-family/commit/6304e19ae68f6f764ea641cc2677215f56faed12))
* **domain:** PersonRepo::set_linked_user_id for invite-accept ([9b9d4b6](https://github.com/BentBr/my-family/commit/9b9d4b6ed3ba7d85004ae643e050501a424557dc))
* **email:** owner-transfer templates (owner + admin sides, en/de) ([1570dae](https://github.com/BentBr/my-family/commit/1570dae50add74f6bc715bd7bb91251be8c3c313))
* **fe:** /account/owner-transfer/confirm page ([bbb01e1](https://github.com/BentBr/my-family/commit/bbb01e1efea267da762064b7691ec33714b8cd58))
* **fe:** /admin routes + admin role guard ([1e422e5](https://github.com/BentBr/my-family/commit/1e422e51b19031c2217353cbad4af655582435ee))
* **fe:** /admin/audit view with filters + server pagination ([d357300](https://github.com/BentBr/my-family/commit/d3573009e1386669fdd0688d4868be2f50e4cbc9))
* **fe:** /admin/invites view + enable rail entry ([e399913](https://github.com/BentBr/my-family/commit/e39991386b7a86be68b35fd444bc65fb7fd23861))
* **fe:** /admin/members view with role-matrix-aware action menu ([454397b](https://github.com/BentBr/my-family/commit/454397b6bbe8f0c5e971612e9d51210caa7f3f24))
* **fe:** 1200px desktop clamp + drop NavDrawer on admin pages ([5531f98](https://github.com/BentBr/my-family/commit/5531f98521fb6be9e87712cea1a71c948680e265))
* **fe:** admin + audit i18n namespace en/de ([0bfa219](https://github.com/BentBr/my-family/commit/0bfa2194d9d4df9afd9a2bc5975d7e288d505aff))
* **fe:** admin layout shell with side-rail ([a8d197a](https://github.com/BentBr/my-family/commit/a8d197a5996a295ebdf0541a255e2a7be4f49a3f))
* **fe:** admin nav entry visible to admin+owner only ([54b4466](https://github.com/BentBr/my-family/commit/54b44662734a91939c7e67d6bf0023f3bd5c6d76))
* **fe:** admin.members i18n keys + member toast entries (en/de) ([7545b1c](https://github.com/BentBr/my-family/commit/7545b1cbd528ca36735f36cd4dc40faa058ad5c7))
* **fe:** audit list TanStack hook + regenerated OpenAPI bundle ([c2ef626](https://github.com/BentBr/my-family/commit/c2ef626142229f57caa4ba1efe43ab595a52f52c))
* **fe:** centralized request layer — unwrap + refresh-failure coverage ([9575ff4](https://github.com/BentBr/my-family/commit/9575ff446cce32c56599857554aa650fa1b2e4f4))
* **fe:** ContactsSection + ContactEdit replace flat person fields ([6001c58](https://github.com/BentBr/my-family/commit/6001c5861eb04df4ac4f668e90afcc015d6dd0cb))
* **fe:** i18n keys for owner transfer + toasts (en/de) ([665fd3c](https://github.com/BentBr/my-family/commit/665fd3c292c8e4a3fad7fec9250dda313a4b6b0e))
* **fe:** invites hooks (list/create/cancel) ([64ed638](https://github.com/BentBr/my-family/commit/64ed638cc2edad91d678cad90dcb34aad6ec699d))
* **fe:** members hooks (list/setRole/revoke) + regenerated OpenAPI ([727bd35](https://github.com/BentBr/my-family/commit/727bd35d03e8c2499931598c074c9db65d179dd2))
* **fe:** owner transfer CTA + pending banner on /admin/members ([1fb54ef](https://github.com/BentBr/my-family/commit/1fb54ef634b9eda3ba5ef947373f50aaed3429bc))
* **fe:** owner-transfer hooks + openapi.json regen ([c73f490](https://github.com/BentBr/my-family/commit/c73f490c79b4af95bb1590b6ffd7587877bdc4c9))
* **fe:** per-user favourites — tree star, detail star, upcoming pill ([8b77157](https://github.com/BentBr/my-family/commit/8b77157210756949535a3a1217b81fa4ff6c3103))
* **fe:** PersonDetail invite-to-family CTA + modal ([4ac602d](https://github.com/BentBr/my-family/commit/4ac602ded77311fdaac9edec21e74b1f756e0aff))
* **fe:** Upcoming dates page + sidebar + filter toggle ([f330487](https://github.com/BentBr/my-family/commit/f33048728e726106af9b15c49d6f158eb8a067a5))
* **fe:** Upcoming dates page + sidebar entry + filter toggle ([77854f5](https://github.com/BentBr/my-family/commit/77854f55d2044d5d42d822d8c942e710367366f9))
* **persistence:** audit_log list_filtered with actor + person resolution ([c874d2e](https://github.com/BentBr/my-family/commit/c874d2e86f975a48154ab404ef0e2eca6bfbe0c3))
* **persistence:** family_invites stores + returns person_id ([603b2b9](https://github.com/BentBr/my-family/commit/603b2b946bad0494440066d07e6f8605524eb3d0))
* **persistence:** impl PersonRepo::set_linked_user_id ([c3ceaaa](https://github.com/BentBr/my-family/commit/c3ceaaa77d1e6348ef40e6ae8e68bc07c0753de7))
* **persistence:** list_with_users joins users for member email + name ([e33ff6b](https://github.com/BentBr/my-family/commit/e33ff6b72fc93b5a32335d22f2a450cb613085a5))
* **persistence:** PgOwnerTransferRepo with begin/confirm/complete/cancel ([aedf52b](https://github.com/BentBr/my-family/commit/aedf52b1a9fe27be820888685d31df6156449eb0))
* **seeder:** 2 more Klaus partners (Karin + Yuki) ([fafc932](https://github.com/BentBr/my-family/commit/fafc932860646ad2c00152e5834678af8cece712))
* **seeder:** person_contacts rows on Klaus, Anna, Hannelore ([6e7f417](https://github.com/BentBr/my-family/commit/6e7f417d85516c3df175a7bdce60bd2eb38d8da7))


### Bug Fixes

* **fe:** AdminLayout uses main NavDrawer + Vuetify rail + back link ([6500184](https://github.com/BentBr/my-family/commit/65001840a350e76ecb6c80752d42c69035913132))
* **fe:** auto-resume invite after sign-in for anonymous invitees ([f91695e](https://github.com/BentBr/my-family/commit/f91695e855f3a74617db91cb75b8536f0108e8fa))
* **fe:** capture ?center= once on mount so drawer + center survive ([26dd0f5](https://github.com/BentBr/my-family/commit/26dd0f5e999e33e34ee3118dcfed92d0f77d5666))
* **fe:** chooseParentBlock walks all members for parent lookup ([fd9b462](https://github.com/BentBr/my-family/commit/fd9b462acf4ffdfec22c25fc1e8d64d08ab19c5a))
* **fe:** deep-link ?center= opens drawer + anniversary rows navigate ([6a8ba77](https://github.com/BentBr/my-family/commit/6a8ba77722644ba6a9978380603d0975a5393665))
* **fe:** deep-link drawer reopens on every visit, not only first ([8a5312b](https://github.com/BentBr/my-family/commit/8a5312b8be758453f82f8a6a0f6018b543c1c9b8))
* **fe:** favourite star always visible, not hover-only ([a6899ba](https://github.com/BentBr/my-family/commit/a6899ba981098e6c3738cada4a5c197736dc8458))
* **fe:** hide invite-to-family CTA when person has linked account ([445444c](https://github.com/BentBr/my-family/commit/445444c9ba301a030f3c32d0f146981506955147))
* **fe:** import Ref type + narrow cast in upcoming view test ([1d6ac54](https://github.com/BentBr/my-family/commit/1d6ac54aa9bda79e9598043e400f4334ed83df06))
* **fe:** PersonDetail surfaces linked-account state via chip ([36d9e0e](https://github.com/BentBr/my-family/commit/36d9e0e7436a06deab45ab27b255ecc750f0f086))
* **fe:** pin partner adjacency in multi-couple blocks ([b36207e](https://github.com/BentBr/my-family/commit/b36207ed58faf2657c13b23b21dc88b9b04e24b2))
* **fe:** poll upcoming-row count instead of synchronous probe ([09ec40b](https://github.com/BentBr/my-family/commit/09ec40b1e8688e9e41b54c755f1c40a77016054d))
* **fe:** replace v-btn-toggle with plain buttons in upcoming filter ([d39760d](https://github.com/BentBr/my-family/commit/d39760d3d0e57aa367d9f55960008100a8272669))
* **fe:** show invitee email + role on invite audit rows ([ca5e196](https://github.com/BentBr/my-family/commit/ca5e19674a314c9fc782fe78ff2d5a50b297123b))
* **fe:** sub-cluster floor uses COL_GAP not CLUSTER_GAP ([709d367](https://github.com/BentBr/my-family/commit/709d367e5618549180e4dfde6beeb9e7d603c8ad))
* **fe:** tree sub-cluster overlap + memorial icon ([bf474c8](https://github.com/BentBr/my-family/commit/bf474c8a8132eed8639836537e38416638628700))
* **fe:** URL-driven deep link + disable-route-watcher on drawer ([e834547](https://github.com/BentBr/my-family/commit/e8345473a9ebbcf95ad61838d837a3a879d78647))
* **fe:** user role can edit their own linked person row ([1747244](https://github.com/BentBr/my-family/commit/1747244af563a4486e4f437981b0b5468fb5e487))
* invite token is the auth factor; no sessionStorage stash ([d614068](https://github.com/BentBr/my-family/commit/d614068a863a0deaf92a9b1295eb228161c6c0a0))

## [0.1.1](https://github.com/BentBr/my-family/compare/core-v0.1.0...core-v0.1.1) (2026-05-22)


### Features

* add cargo workspace manifest with shared lints ([e3c4a77](https://github.com/BentBr/my-family/commit/e3c4a77c7d7d804bb28faf4a0682cad6c7e73bac))
* **api:** /auth/magic-link + /auth/consume + /auth/refresh + /me + /logout ([86106e9](https://github.com/BentBr/my-family/commit/86106e9b6e8bd851af8e7a7234babf494b4f3dae))
* **api:** /families CRUD + /invites/accept with i18n params ([b5863c6](https://github.com/BentBr/my-family/commit/b5863c69cb102d69ee1ec3bc7fbc46bcde0e98d8))
* **api:** /persons, /parent-links, /partnerships, /relationships endpoints ([8472a9e](https://github.com/BentBr/my-family/commit/8472a9e9ff3e1f17e672ee6ca92a7a6a0866ab6e))
* **api:** /users/me CRUD + email-change with old-mail confirm ([7eb3631](https://github.com/BentBr/my-family/commit/7eb3631bab599d6a6d859bc8d9d3ff472bf3c452))
* **api:** ApiError, ErrorCode, RFC 7807 body with sanitized internals ([107643b](https://github.com/BentBr/my-family/commit/107643b787974050b8e2377284da3cc6d2c7c716))
* **api:** ApiResponse/ResponseMeta/Pagination envelope types ([e782682](https://github.com/BentBr/my-family/commit/e782682aee02f35681ac76073189746ff3c46bac))
* **api:** AppState dependency container ([6f6169a](https://github.com/BentBr/my-family/commit/6f6169ad09ec1e7940c806b0c550726a8e37cd8f))
* **api:** auth middleware verifies JWT + resolves X-Family-Id ([af472da](https://github.com/BentBr/my-family/commit/af472da922259a43749a347396cbbb7914ef6340))
* **api:** auth service issuing access JWT with family claims ([674ab10](https://github.com/BentBr/my-family/commit/674ab10d7f6534085bd46d54624c1662197afef5))
* **api:** build_app factory with CORS + middleware stack + health route ([e20872b](https://github.com/BentBr/my-family/commit/e20872b795de0c475ce04de1e4b0548ae64a4913))
* **api:** build_app takes AppState; bin constructs it ([7b69075](https://github.com/BentBr/my-family/commit/7b690758a38eeb358f4648278bc749fc548e7c5f))
* **api:** deterministic seed for dev + test + CI ([a05eabe](https://github.com/BentBr/my-family/commit/a05eabe7cd547b4af6291009666f6df360a8ffd9))
* **api:** expose linked_user_id on tree nodes for FE self-center ([e30c219](https://github.com/BentBr/my-family/commit/e30c219619154ff20602cfd6e07ed036abb1bfe4))
* **api:** expose partnership id/kind/dates + parent kind on tree ([3f1e553](https://github.com/BentBr/my-family/commit/3f1e5536831b3bf9d345aa93f2d7b88d4b30e225))
* **api:** extend AppState with person/parent_link/partnership repos + tree builder ([2c6de65](https://github.com/BentBr/my-family/commit/2c6de65b3540ad3897cda4606412995d428c45a6))
* **api:** FieldViolation.params for FE i18n placeholder substitution ([45d4c8b](https://github.com/BentBr/my-family/commit/45d4c8ba26411667f5c329bd5f252f8736729a69))
* **api:** gen-jwt-keys helper emits Ed25519 keypair for .env ([b6b782d](https://github.com/BentBr/my-family/commit/b6b782d3d1064b91ec30253e3e7c59b803c3d3a1))
* **api:** GET /api/v1/health with envelope and request id ([16f915e](https://github.com/BentBr/my-family/commit/16f915e6a97a062261fb0fc85899134e5a36e058))
* **api:** GET /upcoming endpoint with filter ([0fcd341](https://github.com/BentBr/my-family/commit/0fcd341398c6d4e2c5e4b11969b50961622c54f3))
* **api:** jwt module with ed25519 keyset + claims + tokens ([db7dd7d](https://github.com/BentBr/my-family/commit/db7dd7d4e442d41fe9606629248e842ba5063639))
* **api:** main binary loads config, inits tracing, serves build_app ([ffe9f98](https://github.com/BentBr/my-family/commit/ffe9f98eaf3481b3468a923d53acbd0bf7f10102))
* **api:** mount swagger ui at /api/docs when api_enable_docs ([f6cb781](https://github.com/BentBr/my-family/commit/f6cb78110d4f60e89311290b76e9b9c9148ea33f))
* **api:** PanicCatcher middleware converts panics into sanitized internal errors ([f795dc4](https://github.com/BentBr/my-family/commit/f795dc4e2c29aa6882b8f22816c62cece1bae8d1))
* **api:** persons contact + email-from-linked-user sync ([fa01d8c](https://github.com/BentBr/my-family/commit/fa01d8cbc2cb1bd845c3ec850d156abefd05bff8))
* **api:** persons contact + email-from-linked-user sync ([1bf4bf3](https://github.com/BentBr/my-family/commit/1bf4bf339e19d583764f2433febb6f5d26960bcd))
* **api:** reject duplicate parent_link with 409 ([8445874](https://github.com/BentBr/my-family/commit/844587482b2b8583dcba2a5605739c2c5a8bcd00))
* **api:** RequestId middleware with X-Request-Id passthrough ([5fef823](https://github.com/BentBr/my-family/commit/5fef823f4589742ccf91df4ef326eb06f3f137b7))
* **api:** tracing subscriber init (pretty/json formats) ([16f718a](https://github.com/BentBr/my-family/commit/16f718a9d5919161a8d1b99cd3c95e5f986521b1))
* **api:** typed Config loader with env-only inputs and validation ([5683d42](https://github.com/BentBr/my-family/commit/5683d429392da9d48739cebb7d4d1a8839330fea))
* **api:** typed cookie builders for access + refresh ([6bca243](https://github.com/BentBr/my-family/commit/6bca2437fb5877bc74e27cf2ee5238b63b48634b))
* **api:** unique operation_ids on persons/parent-links/partnerships/families ([7244049](https://github.com/BentBr/my-family/commit/7244049470853c12bd5682956d2daf982bd235d1))
* **api:** validation namespace + cycle DB trigger + seed enrich ([d56f8d3](https://github.com/BentBr/my-family/commit/d56f8d3dad55f0e70600b588306da0a179ff0fec))
* **cache:** Redis pool wrapper with ping smoke test ([dc40969](https://github.com/BentBr/my-family/commit/dc40969c357a1767c6e033521cf1b64be596f8d3))
* **cache:** redis sliding-window rate limiter ([e685433](https://github.com/BentBr/my-family/commit/e685433f5ae7861e91944524530ffc5bebb06802))
* **ci:** tighten clippy to pedantic + nursery + all-features ([d8d3a62](https://github.com/BentBr/my-family/commit/d8d3a6223e2aa58d86ef1ce01548e48090f80277))
* docker compose with postgres 18, redis 7, mailpit ([0ab0e9a](https://github.com/BentBr/my-family/commit/0ab0e9a3da155b36eb845ad71e5ec6e36e2c755b))
* **domain:** persons/parent_links/partnerships repo traits + cycle detection ([add6b99](https://github.com/BentBr/my-family/commit/add6b99dafc7f0164d5384b8c5eb0829d3473fe6))
* **domain:** repository traits for auth + families ([c8fe51e](https://github.com/BentBr/my-family/commit/c8fe51ef466c2412e93f8f18d99098a93382e3f1))
* **domain:** Role enum and static Capability sets ([11e6ff7](https://github.com/BentBr/my-family/commit/11e6ff7692518fb6f0a6be5dd8b52a44e54aa634))
* **domain:** typed Uuid newtypes for UserId/FamilyId/PersonId/FamilyMembershipId ([c2f85e7](https://github.com/BentBr/my-family/commit/c2f85e7a33d6c43def85a5479944b5577fab09ea))
* **email:** askama email-change templates (en+de) ([fdabe80](https://github.com/BentBr/my-family/commit/fdabe808331691ff562f6141c746d4f19a9d6482))
* **email:** askama templates for magic-link + invite (en + de) ([40ff9dd](https://github.com/BentBr/my-family/commit/40ff9dd91950fec63a764b11635f4d06f197e10a))
* **email:** EmailSender trait, lettre SMTP impl, in-memory Fake ([ff7651e](https://github.com/BentBr/my-family/commit/ff7651e593afb5419caff314dc6fc9b1390bf2ee))
* **fe:** AccountView + user menu in AppBar ([df0dd21](https://github.com/BentBr/my-family/commit/df0dd21f25dd1c4a2db711babe697e38c93d1d50))
* **fe:** add knip stale-code detection + clean up unused exports/deps ([41ac891](https://github.com/BentBr/my-family/commit/41ac891420dccce6c8308f46f30bee7bd1bd8a6d))
* **fe:** always-active-family UX + /tree fallback to /families/create ([53e127a](https://github.com/BentBr/my-family/commit/53e127a648059c3785c2d19a340883d523db26b3))
* **fe:** amber outline for current-user TreeNode ([c5d7dec](https://github.com/BentBr/my-family/commit/c5d7dec6da36f4f018473c2a640ced8c4c886e6a))
* **fe:** branded id types + commit openapi.json contract ([e40c1af](https://github.com/BentBr/my-family/commit/e40c1af9af203bfcede4d94078e749af4d075a80))
* **fe:** design tokens, SmartIcon, LoginLayout + MainLayout, fade transitions ([c0cc291](https://github.com/BentBr/my-family/commit/c0cc291cf4caa60e2acf0075c86ca7a6eda36860))
* **fe:** eslint flat config with strict typescript + prettier ([ddaa527](https://github.com/BentBr/my-family/commit/ddaa527e99f9b1f6ab7822c994eb3145c881cb03))
* **fe:** ex-spouse adjacency in tree layout blocks ([f575b43](https://github.com/BentBr/my-family/commit/f575b439000d99ac065f6a98ccf56a72b5a43eeb))
* **fe:** family create + picker + invite-accept views with router guard ([c9ff8b2](https://github.com/BentBr/my-family/commit/c9ff8b21a2d37fb43182c28f67328ee6f5387144))
* **fe:** family switcher "create new" + empty-tree CTA ([19fdbab](https://github.com/BentBr/my-family/commit/19fdbab870be9e9ce0fa618bd8e631babe39f600))
* **fe:** FamilyTree SVG component with pan/zoom and a11y-friendly nodes ([e5056c7](https://github.com/BentBr/my-family/commit/e5056c72bc5a304c62e73f70926430191111f8bc))
* **fe:** full-field PersonEdit + contact section ([a67386e](https://github.com/BentBr/my-family/commit/a67386eec9f6d662ff19d03175e276012b879cf8))
* **fe:** global toast UI for unexpected backend errors ([ee16b29](https://github.com/BentBr/my-family/commit/ee16b29fd0ebfb5801ae28c1915609084f24ab61))
* **fe:** hover highlights direct relations + connecting edges ([373bbfc](https://github.com/BentBr/my-family/commit/373bbfc1c03a1af2e9bf7e4bf3a1b6bc86bc80c5))
* **fe:** locale store synced with auth + /users/me hooks ([e02c3a5](https://github.com/BentBr/my-family/commit/e02c3a5167385ce785e6d426e4ebf48723356f45))
* **fe:** Login + Consume + Health views with Vuetify primitives ([3a58651](https://github.com/BentBr/my-family/commit/3a5865195ced40f4c15ba439cac8cbe4aeca9fb4))
* **fe:** logout redirects to /auth/sign-in; /invite/* gate exempt ([3f65054](https://github.com/BentBr/my-family/commit/3f650549f7435507d1cea33d573b661590c87689))
* **fe:** logout wipes all my-family:* local + session storage ([adf06cb](https://github.com/BentBr/my-family/commit/adf06cbe78847075d7bd3dc7c623671420b3cf3a))
* **fe:** mid-session 401 redirects to /auth/sign-in ([16adc5d](https://github.com/BentBr/my-family/commit/16adc5d9dfc4e3d72bf2b286b8906826f0cd1800))
* **fe:** openapi-fetch client with family-id + auth-refresh + error-translation interceptors ([0fed3d9](https://github.com/BentBr/my-family/commit/0fed3d99ff064b920eb7bbb62ee5ff8d14e8466d))
* **fe:** parent/partner kind selectors + tree fit-to-view on mount ([86c903d](https://github.com/BentBr/my-family/commit/86c903d0a30ebaaa76f66242c5134262f3cfec9a))
* **fe:** PersonDetail full fields + role-gated edit + inline relations ([ee06b89](https://github.com/BentBr/my-family/commit/ee06b896c4a86cde5a3a805d543c592c6324d234))
* **fe:** persons + relationships TanStack Query hooks ([55801fa](https://github.com/BentBr/my-family/commit/55801faa3bfef9149dfa2e3c7d4417690226e0bc))
* **fe:** pinia stores (auth, activeFamily, locale, ui) ([76d6091](https://github.com/BentBr/my-family/commit/76d6091491b0f1bd76977626c8d902ca56c98e62))
* **fe:** pnpm + vite + vue 3 + strict ts scaffold ([d88b779](https://github.com/BentBr/my-family/commit/d88b77929532bfe363881e5e760ea65dbc05f233))
* **fe:** readable zoom defaults; clamp Fit to view ([58bda88](https://github.com/BentBr/my-family/commit/58bda88c6e3211cae36b66004a2698af757e3c48))
* **fe:** real auth store + hooks; router guard; consume redirects to /health ([cbf7be2](https://github.com/BentBr/my-family/commit/cbf7be281969ea91b097517b4e194d291926b6eb))
* **fe:** router, vue-i18n (en/de), TanStack Query wired in main ([0acccd4](https://github.com/BentBr/my-family/commit/0acccd4c94c3474c5d7764932035dfdb607bf924))
* **fe:** show current age (or age at death) on each TreeNode ([0eb738a](https://github.com/BentBr/my-family/commit/0eb738a77139c9780a81a4441354c03bcf619e49))
* **fe:** split birth/death dates onto two TreeNode rows ([c7c6569](https://github.com/BentBr/my-family/commit/c7c65692dd95001381d73cc136e119b87b9539fd))
* **fe:** success toasts on profile/family/invite/email mutations ([fdcca80](https://github.com/BentBr/my-family/commit/fdcca80c15ac99d52a2cdbe2898f06a211e5499a))
* **fe:** translate validation errors + tree-view fixes ([01c950b](https://github.com/BentBr/my-family/commit/01c950b38b183bb322b8734cc6925b75f19474ae))
* **fe:** tree layout — generation rows + partnership pairing ([1c1b197](https://github.com/BentBr/my-family/commit/1c1b197fc71aa62c15cbef83ca48aa4e0f6d637a))
* **fe:** tree layout v2 — partners adjacent + sibling clusters ([a88e092](https://github.com/BentBr/my-family/commit/a88e092c46b8d03fdd30ab1ff5b194c3b9d7523a))
* **fe:** tree layout v3 — top-down gen + partner equalize ([c2da706](https://github.com/BentBr/my-family/commit/c2da706c2aace8e1269079c4204d37cc3dfe3a19))
* **fe:** tree view + person detail drawer + edit form ([cae443b](https://github.com/BentBr/my-family/commit/cae443bbfc63d4ce85baacda24f6375c0cb8d28f))
* **fe:** useGetPerson + partnership/parent-link mutation hooks ([a9703f0](https://github.com/BentBr/my-family/commit/a9703f0b8b4e0e68d35e0d2f2cb0769a83dacd9d))
* **fe:** UX polish for active-family + redirects + person form ([78a0e7c](https://github.com/BentBr/my-family/commit/78a0e7c2c582231c39554c11b4b7d0c19cd6f9a2))
* **infra:** expose compose services via *.my-family.docker network aliases ([e5ddcb7](https://github.com/BentBr/my-family/commit/e5ddcb762491f60188408f14582ded59678fd632))
* **migrator:** run_migrations binary with status/check/dry-run/target flags ([3ad83e1](https://github.com/BentBr/my-family/commit/3ad83e170207ec7aed9a7b836341327d0afded59))
* **openapi:** ApiDoc aggregator with health path and shared schemas ([d66bc6e](https://github.com/BentBr/my-family/commit/d66bc6e541a3a107e1ac0ab3318fb800cfc88c1c))
* **openapi:** expose auth/families/invites paths and schemas ([8b96df6](https://github.com/BentBr/my-family/commit/8b96df66f196c9c87c10875209fc5848a932e3c9))
* **openapi:** persons / relationships paths and schemas ([fd21450](https://github.com/BentBr/my-family/commit/fd21450d0a4c16444a26e0069b1771b701d48575))
* **openapi:** security scheme on every authenticated endpoint ([0343175](https://github.com/BentBr/my-family/commit/0343175de9beccb44bc7e7a67294746686cf7703))
* **persistence:** 0001 migration for auth + families schema ([86c692e](https://github.com/BentBr/my-family/commit/86c692e478249b54c1e5ce1db3156b0046075445))
* **persistence:** 0002 migration — persons, parent_links, partnerships ([3f37b79](https://github.com/BentBr/my-family/commit/3f37b79ec6913f2a2008c4a34e01301462b2e37e))
* **persistence:** Postgres pool wrapper with ping smoke test ([c534eda](https://github.com/BentBr/my-family/commit/c534eda0e3521574b47f08ae573368496c595a46))
* **persistence:** postgres repos for auth + families ([daebb5e](https://github.com/BentBr/my-family/commit/daebb5e8653daf1cf29f38073d13970caa6e1dbc))
* **persistence:** postgres repos for persons, parent_links, partnerships ([bbe5a9e](https://github.com/BentBr/my-family/commit/bbe5a9eaa8a0f9c822fd665adfa9a0ee44604cab))
* **reminder-worker:** minimal boot that connects to db+redis and idles ([a7dee8a](https://github.com/BentBr/my-family/commit/a7dee8a4c5a716a9eeccaf2426619b6804483467))
* scaffold eight workspace crates with minimal stubs ([854907f](https://github.com/BentBr/my-family/commit/854907f91bdcff052b4183e6a5bb15fb5dabd514))
* **seeder:** expand to 20 persons covering relationship edge cases ([2501bf1](https://github.com/BentBr/my-family/commit/2501bf164c8a3b2859b2d92c520e9f29223a0544))


### Bug Fixes

* **api:** clippy hygiene on upcoming integration test ([f58934b](https://github.com/BentBr/my-family/commit/f58934bdb49b836c633ea8a08cce4dd268d06cf8))
* **api:** restore long EdgePair/PartnerEdge first-paragraph + sync openapi.json ([434385e](https://github.com/BentBr/my-family/commit/434385e542448075cca32c803d82cadbe78b0c91))
* **api:** shorten doc first-paragraph on tree edge types ([5a6b094](https://github.com/BentBr/my-family/commit/5a6b094ec59e746e1350258b06c3835348b56f28))
* **api:** shorten persons_contact docs to satisfy clippy ([b94049d](https://github.com/BentBr/my-family/commit/b94049d80eccb92f48cf2f982537df5701b573fa))
* **api:** stricter email syntax check (local part + tld letters) ([2381e9f](https://github.com/BentBr/my-family/commit/2381e9f62db259550740fc1909e2f99b7a515d98))
* **api:** sync EdgePair/PartnerEdge doc comments with openapi.json ([aadb2fd](https://github.com/BentBr/my-family/commit/aadb2fd1283dc1eee799d1ed2fd4ed8e2763eef2))
* **api:** unique operation_id for /users/me to avoid FE codegen clash ([138cd02](https://github.com/BentBr/my-family/commit/138cd02cf8063fa395ede525cb70abf192654447))
* **ci:** e2e against a production build, not pnpm dev ([0f492a0](https://github.com/BentBr/my-family/commit/0f492a050f76c914101976485e47f9bf45bd599b))
* e2e routing + logout cookie domain + secure-context-safe toast IDs ([51d9d3b](https://github.com/BentBr/my-family/commit/51d9d3b7b53b8937484d4361ed08154e030de610))
* **e2e:** drill into &lt;input&gt; inside the v-text-field for sign-in email ([a073139](https://github.com/BentBr/my-family/commit/a0731393c39fd9c3345c7b6efbca725c99ce1627))
* **email:** backtick env var names in doc comment so clippy::doc_markdown passes ([f3b891c](https://github.com/BentBr/my-family/commit/f3b891cd3723bcf278ea2e1a5ce93f6f1557984c))
* **fe:** ConsumeView dedupes magic-link tokens across re-mounts ([c85dc3a](https://github.com/BentBr/my-family/commit/c85dc3ab35c1037ccc0a6d2075fb931b0c926a12))
* **fe:** drop SVG viewBox so g transform owns scale alone ([cab543b](https://github.com/BentBr/my-family/commit/cab543b67d40b113ed1407df8f9e8a1d95bec4b3))
* **fe:** e2e global hooks honor REDIS_URL / DATABASE_URL in CI ([b4aaa84](https://github.com/BentBr/my-family/commit/b4aaa84c67cf73e1affddf5a39a0924ec6265d28))
* **fe:** InviteAccept setActive + nextTick flush before nav (router 5) ([7071ccb](https://github.com/BentBr/my-family/commit/7071ccbedf261fc51cd5fd9fbdc5c57c49efbaa4))
* **fe:** make ConsumeView idempotent (prevent double token consumption) ([d1f48d5](https://github.com/BentBr/my-family/commit/d1f48d5f8465d2c5f4d6b07b4315546cdd9f9f32))
* **fe:** partnership POST uses partner_a_id / partner_b_id ([e23e686](https://github.com/BentBr/my-family/commit/e23e686d7ced8194bae1a825dcc3cf5851153282))
* **fe:** rank tree generations by descendant depth + age ([7b1f120](https://github.com/BentBr/my-family/commit/7b1f1208da6b3143874bb3e3875470717f2f444e))
* **fe:** tree fit-to-view wins over center-on-self on initial mount ([23730b5](https://github.com/BentBr/my-family/commit/23730b576c685c67d63e1f0843476f99f4302dbe))
* **fe:** tree layout drops nodes on multi-pair midpoint collision ([87f83af](https://github.com/BentBr/my-family/commit/87f83af689ef8e69773c3726a78932de6a0ab4f2))
* **fe:** vue-i18n 11 generics + vite-env types so typecheck passes ([3bf6b46](https://github.com/BentBr/my-family/commit/3bf6b468bc8eba427a06c2287b134ac7acf050b2))
* **openapi:** backtick utoipa::OpenApi in module doc ([0cbb3ce](https://github.com/BentBr/my-family/commit/0cbb3ce809b43297881d34a049f7a54e77a29e47))
* **openapi:** empty-body openapi-dump stub (no clippy warnings) ([ab7ff59](https://github.com/BentBr/my-family/commit/ab7ff59b4475068025bfed3a1c71dc6530e6c3f8))
* **persistence:** drop testcontainer at test end so docker reaps it ([0dff0b8](https://github.com/BentBr/my-family/commit/0dff0b8f4df37f79df422a58d6524d8f7077b5c1))
* **rdt:** making sure the config is correct ([1d448a0](https://github.com/BentBr/my-family/commit/1d448a0a4b95835760447b6466d85bcbcb48ed2d))
