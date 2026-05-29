// Backend renders email links from `WEB_PUBLIC_URL` (= `http://my-fam-tree.docker`).
// That URL works for a host-side browser (dinghy nginx forwards port 80 → fe:5173)
// but the in-network Playwright browser bypasses dinghy and must hit the FE
// directly on `:5173`. We normalize the link to the active Playwright baseURL,
// which is set per environment (default = host dinghy URL; compose = `:5173`).
const E2E_BASE_URL = process.env['E2E_BASE_URL'] ?? 'http://my-fam-tree.docker'

export function rewriteEmailLink(link: string): string {
    return link.replace(/^https?:\/\/my-fam-tree\.docker(?::\d+)?/, E2E_BASE_URL)
}
