export interface DesignMdSpec {
  /** Original markdown 源（用于往返保真度） */
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
