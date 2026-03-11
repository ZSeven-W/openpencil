/**
 * LinkedIn format presets for V1.
 */

export interface FormatPreset {
  id: string
  name: string
  platform: 'linkedin'
  width: number
  height: number
  contentType: 'carousel' | 'video' | 'post'
}

export const LINKEDIN_CAROUSEL: FormatPreset = {
  id: 'linkedin-carousel',
  name: 'LinkedIn Carousel',
  platform: 'linkedin',
  width: 1080,
  height: 1350,
  contentType: 'carousel',
}

export const LINKEDIN_VIDEO: FormatPreset = {
  id: 'linkedin-video',
  name: 'LinkedIn Video',
  platform: 'linkedin',
  width: 1080,
  height: 1920,
  contentType: 'video',
}

export const LINKEDIN_POST: FormatPreset = {
  id: 'linkedin-post',
  name: 'LinkedIn Post',
  platform: 'linkedin',
  width: 1200,
  height: 1200,
  contentType: 'post',
}

export const FORMAT_PRESETS: FormatPreset[] = [
  LINKEDIN_CAROUSEL,
  LINKEDIN_VIDEO,
  LINKEDIN_POST,
]

export const DEFAULT_FORMAT = LINKEDIN_CAROUSEL
