export interface DesignMdSpec {
  /** Original markdown source (for round-trip fidelity) */
  raw: string;
  projectName?: string;
  visualTheme?: string;
  colorPalette?: DesignMdColor[];
  typography?: DesignMdTypography;
  componentStyles?: string;
  layoutPrinciples?: string;
  generationNotes?: string;
}

export interface DesignMdColor {
  name: string;
  hex: string;
  role: string;
}

export interface DesignMdTypography {
  fontFamily?: string;
  headings?: string;
  body?: string;
  scale?: string;
}
