import fs from 'fs';
import path from 'path';

describe('Issue #1067: UI Package Entry Point', () => {
  describe('Package configuration', () => {
    test('should have main entry point defined in package.json', () => {
      const packageJsonPath = path.resolve(__dirname, '../package.json');
      const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));

      expect(packageJson.main).toBeDefined();
      expect(packageJson.main).toBe('dist/index.js');
    });

    test('should have types entry point defined in package.json', () => {
      const packageJsonPath = path.resolve(__dirname, '../package.json');
      const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));

      expect(packageJson.types).toBeDefined();
      expect(packageJson.types).toBe('dist/index.d.ts');
    });

    test('should have root index.ts file', () => {
      const indexPath = path.resolve(__dirname, 'index.ts');
      expect(fs.existsSync(indexPath)).toBe(true);
    });
  });

  describe('TypeScript build configuration', () => {
    test('should have tsconfig.build.json in ui directory', () => {
      const tsconfigPath = path.resolve(__dirname, '../tsconfig.build.json');
      expect(fs.existsSync(tsconfigPath)).toBe(true);
    });

    test('should include components in TypeScript build', () => {
      const tsconfigPath = path.resolve(__dirname, '../tsconfig.build.json');
      const tsconfig = JSON.parse(fs.readFileSync(tsconfigPath, 'utf-8'));

      const includes = tsconfig.include || [];
      const hasComponents = includes.some((pattern: string) =>
        pattern.includes('components')
      );

      expect(hasComponents).toBe(true);
    });

    test('should include hooks in TypeScript build', () => {
      const tsconfigPath = path.resolve(__dirname, '../tsconfig.build.json');
      const tsconfig = JSON.parse(fs.readFileSync(tsconfigPath, 'utf-8'));

      const includes = tsconfig.include || [];
      const hasHooks = includes.some((pattern: string) =>
        pattern.includes('hooks')
      );

      expect(hasHooks).toBe(true);
    });

    test('should include src in TypeScript build', () => {
      const tsconfigPath = path.resolve(__dirname, '../tsconfig.build.json');
      const tsconfig = JSON.parse(fs.readFileSync(tsconfigPath, 'utf-8'));

      const includes = tsconfig.include || [];
      const hasSrc = includes.some((pattern: string) =>
        pattern.includes('src')
      );

      expect(hasSrc).toBe(true);
    });
  });

  describe('Build output validation', () => {
    test('hooks directory should be compiled to dist', () => {
      // Note: This test validates that hooks/index.ts would be compiled
      // The actual dist folder is generated during build, but we verify the source exists
      const hooksPath = path.resolve(__dirname, '../hooks');
      expect(fs.existsSync(hooksPath)).toBe(true);
    });

    test('hooks index file should exist', () => {
      const hooksIndexPath = path.resolve(__dirname, '../hooks/index.ts');
      expect(fs.existsSync(hooksIndexPath)).toBe(true);
    });

    test('components directory should exist for build', () => {
      const componentsPath = path.resolve(__dirname, '../components');
      expect(fs.existsSync(componentsPath)).toBe(true);
    });
  });

  describe('Package exports', () => {
    test('should export from components', () => {
      const indexPath = path.resolve(__dirname, 'index.ts');
      const content = fs.readFileSync(indexPath, 'utf-8');

      expect(content).toMatch(/export.*components/i);
    });

    test('should export from hooks', () => {
      const indexPath = path.resolve(__dirname, 'index.ts');
      const content = fs.readFileSync(indexPath, 'utf-8');

      expect(content).toMatch(/export.*hooks/i);
    });

    test('root index.ts should have valid exports', () => {
      const indexPath = path.resolve(__dirname, 'index.ts');
      const content = fs.readFileSync(indexPath, 'utf-8');

      // Should have at least some export statements
      expect(content).toMatch(/export\s+/);
    });
  });

  describe('Build configuration requirements', () => {
    test('tsconfig.build.json should specify correct output directory', () => {
      const tsconfigPath = path.resolve(__dirname, '../tsconfig.build.json');
      const tsconfig = JSON.parse(fs.readFileSync(tsconfigPath, 'utf-8'));

      expect(tsconfig.compilerOptions).toBeDefined();
      expect(tsconfig.compilerOptions.outDir).toBe('dist');
    });

    test('tsconfig.build.json should specify correct module format', () => {
      const tsconfigPath = path.resolve(__dirname, '../tsconfig.build.json');
      const tsconfig = JSON.parse(fs.readFileSync(tsconfigPath, 'utf-8'));

      expect(tsconfig.compilerOptions).toBeDefined();
      // Should target module system that generates dist/index.js
      expect(tsconfig.compilerOptions.module).toBeTruthy();
    });

    test('package.json should not include node_modules in files', () => {
      const packageJsonPath = path.resolve(__dirname, '../package.json');
      const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));

      expect(packageJson.files).toBeDefined();
      expect(packageJson.files).toContain('dist');
      expect(packageJson.files).not.toContain('node_modules');
    });
  });
});
