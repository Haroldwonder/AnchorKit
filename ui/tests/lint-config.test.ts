/**
 * Lint configuration validation test
 * Issue #1070: Ensure ESLint glob pattern covers all TypeScript files
 */

import * as fs from 'fs';
import * as path from 'path';

describe('Lint Configuration', () => {
  it('should have eslint script in package.json', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));

    expect(packageJson.scripts.lint).toBeDefined();
    expect(typeof packageJson.scripts.lint).toBe('string');
  });

  it('should include components directory in lint pattern', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));

    expect(packageJson.scripts.lint).toContain('components');
  });

  it('should include hooks directory in lint pattern', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));

    expect(packageJson.scripts.lint).toContain('hooks');
  });

  it('should include src/config directory in lint pattern', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));

    expect(packageJson.scripts.lint).toContain('src');
  });

  it('should include .storybook directory in lint pattern', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));

    expect(packageJson.scripts.lint).toContain('storybook');
  });
});
