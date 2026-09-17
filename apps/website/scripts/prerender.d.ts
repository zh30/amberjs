export type SeoPage = {
  title: string;
  description: string;
  canonical: string;
  body: string;
  jsonLd?: string;
};

export function renderRoute(pathname: string): SeoPage | null;
export function applyPage(template: string, page: SeoPage | null): string;
export function prerender(): void;
