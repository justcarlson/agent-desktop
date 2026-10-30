import { createProxy } from "@vercel/geistdocs/proxy";
import { config as geistdocsConfig } from "@/lib/geistdocs/config";
import { trackMdRequest } from "@/lib/geistdocs/md-tracking";
import { source } from "@/lib/geistdocs/source";

const proxy = createProxy({
  config: geistdocsConfig,
  markdownRoutes: [{ from: "/desktop/*path", to: "/[lang]/desktop/llms.mdx/*path" }],
  trackMarkdownRequest: trackMdRequest,
  before: ({ request }) => {
    const pathname = request.nextUrl.pathname;
    if (
      Object.keys(geistdocsConfig.translations ?? {}).some(
        (lang) => pathname === `/${lang}` || pathname.startsWith(`/${lang}/`)
      )
    ) {
      return new Response(null, { status: 404 });
    }
    return null;
  },
  after: ({ request }) => {
    const pathname = request.nextUrl.pathname;
    const match = pathname.match(/^\/desktop(?:\/(.*))?$/);
    if (!match) return null;
    const [, rawPath = ""] = match;
    const path = rawPath.replace(/\/$/, "");
    if (
      [".well-known/mcp.json", "agents.md", "llms.txt", "openapi.json", "rss.xml", "sitemap.md", "sitemap.xml", "robots.txt"].includes(path) ||
      path.startsWith("og/") || path === "llms.mdx" || path.startsWith("llms.mdx/")
    ) return null;
    if (
      !source.getPage(path ? path.split("/") : [], geistdocsConfig.defaultLanguage)
    ) {
      return new Response(null, { status: 404 });
    }
    return null;
  },
});

export const config = {
  matcher: [
    "/((?!vc-ap-[^/]+/|desktop/api(?:/|$)|desktop/favicon.ico|api(?:/|$)|_next/static|_next/image|favicon.ico|sitemap.xml|robots.txt).*)",
  ],
};

export default proxy;
