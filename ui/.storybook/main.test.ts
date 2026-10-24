import type { StorybookConfig } from '@storybook/react-vite';
import config from './main';
import fs from 'fs';
import path from 'path';

describe('Storybook Configuration', () => {
  describe('#1065: Story file discovery', () => {
    test('should include stories from root storybook directory', () => {
      const stories = config.stories as string[];
      expect(stories).toContain('../../storybook/**/*.stories.@(ts|tsx)');
    });

    test('should include stories from ui/components directory', () => {
      const stories = config.stories as string[];
      const hasComponentStories = stories.some(pattern =>
        pattern.includes('components') && pattern.includes('stories')
      );
      expect(hasComponentStories).toBe(true);
    });

    test('should load AnchorCapabilityCard story from components directory', () => {
      const componentStoriesPath = path.resolve(
        __dirname,
        '../components/AnchorCapabilityCard.stories.tsx'
      );
      expect(fs.existsSync(componentStoriesPath)).toBe(true);
    });

    test('should load TransactionTimeline story from components directory', () => {
      const componentStoriesPath = path.resolve(
        __dirname,
        '../components/TransactionTimeline.stories.tsx'
      );
      expect(fs.existsSync(componentStoriesPath)).toBe(true);
    });

    test('stories pattern should support both ts and tsx extensions', () => {
      const stories = config.stories as string[];
      const pattern = stories[0];
      expect(pattern).toMatch(/@\(ts\|tsx\)/);
    });

    test('should not have duplicate story file loading', () => {
      const storybookDir = path.resolve(__dirname, '../../storybook');
      const componentsDir = path.resolve(__dirname, '../components');

      const storybookStories = fs
        .readdirSync(storybookDir, { recursive: true })
        .filter((f: string) => f.toString().endsWith('.stories.tsx'));

      const componentStories = fs
        .readdirSync(componentsDir, { recursive: true })
        .filter((f: string) => f.toString().endsWith('.stories.tsx'));

      const storybookNames = new Set(
        storybookStories.map((f: string) => path.basename(f.toString()))
      );
      const componentNames = new Set(
        componentStories.map((f: string) => path.basename(f.toString()))
      );

      const duplicates = [...storybookNames].filter(name => componentNames.has(name));
      if (duplicates.length > 0) {
        expect(duplicates).toEqual([]);
      }
    });
  });

  describe('Storybook framework configuration', () => {
    test('should use @storybook/react-vite framework', () => {
      expect(config.framework?.name).toBe('@storybook/react-vite');
    });

    test('should include essential addons', () => {
      const addons = config.addons as string[];
      expect(addons).toContain('@storybook/addon-essentials');
      expect(addons).toContain('@storybook/addon-interactions');
    });
  });
});
