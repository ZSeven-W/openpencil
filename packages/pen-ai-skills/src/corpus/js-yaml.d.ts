// Local ambient declaration for `js-yaml`. The package ships without
// types; adding `@types/js-yaml` as a dev dep would churn the root
// package.json for one call site. Keep the surface minimal — just the
// one function `corpus-loader.ts` uses. Extend as needed.
declare module 'js-yaml' {
  export function load(s: string): unknown;
}
