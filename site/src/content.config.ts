import { defineCollection } from 'astro:content';
import { docsSchema } from '@astrojs/starlight/schema';
import { guideLoader } from '../guide.mjs';

export const collections = {
  docs: defineCollection({ loader: guideLoader(), schema: docsSchema() }),
};
