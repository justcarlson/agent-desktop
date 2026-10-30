import type {
  GeistdocsAgentReadinessConfig,
  GeistdocsAIConfig,
  GeistdocsNavbarBrand,
} from "@vercel/geistdocs/config";

export const navbarBrand: GeistdocsNavbarBrand = "labs";

export const Logo = () => (
  <span className="font-semibold text-gray-1000 text-lg leading-none tracking-[-3%]">
    agent-desktop
  </span>
);

export const github = {
  branch: "main",
  editPath: "apps/docs/content/docs/{path}",
  owner: "vercel-labs",
  repo: "agent-desktop",
};

export const nav = [
  {
    label: "Docs",
    href: "/desktop",
  },
  {
    label: "npm",
    href: "https://www.npmjs.com/package/agent-desktop",
  },
];

export const suggestions = [
  "How do I install agent-desktop?",
  "What platforms does agent-desktop support?",
];

export const title = "agent-desktop";

export const prompt =
  "Answer questions about agent-desktop using the documentation.";

export const ai = {
} satisfies GeistdocsAIConfig;

export const agent = {
  product: {
    name: "Geistdocs",
    description:
      "Geistdocs is a package-backed documentation system for creating Next.js and Fumadocs sites with shared Vercel documentation patterns.",
    category: "Documentation",
    audience: ["Documentation authors", "Developer experience teams"],
    useCases: [
      "Create package-backed documentation sites",
      "Expose docs as AI-readable Markdown",
      "Share Vercel docs UI and runtime behavior across projects",
    ],
  },
  api: {
    siteOpenApiUrl: "/openapi.json",
  },
  links: [
    {
      label: "Geistdocs source",
      href: `https://github.com/${github.owner}/${github.repo}`,
      description: "Source repository for the Geistdocs package and template",
    },
  ],
} satisfies GeistdocsAgentReadinessConfig;

export const translations = {
  en: {
    displayName: "English",
  },
  cn: {
    displayName: "Chinese",
    search: "搜尋文檔",
  },
};

export const basePath: string | undefined = undefined;

/**
 * Unique identifier for this site, used in markdown request tracking analytics.
 * Each site using geistdocs should set this to a unique value (e.g. "ai-sdk-docs", "next-docs").
 */
export const siteId: string | undefined = undefined;
