/**
 * Storybook dependency version consistency test
 * Issue #1072: Ensure all Storybook packages use consistent versions
 */

import * as fs from 'fs';
import * as path from 'path';

describe('Storybook Dependencies', () => {
  it('should have @storybook packages as devDependencies', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
    const devDeps = packageJson.devDependencies;

    expect(devDeps['@storybook/react']).toBeDefined();
    expect(devDeps['@storybook/addon-essentials']).toBeDefined();
    expect(devDeps['@storybook/addon-interactions']).toBeDefined();
    expect(devDeps['@storybook/react-vite']).toBeDefined();
    expect(devDeps['storybook']).toBeDefined();
  });

  it('@storybook/react should be version ^8.0.0 (matching other storybook packages)', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
    const devDeps = packageJson.devDependencies;

    expect(devDeps['@storybook/react']).toBe('^8.0.0');
  });

  it('@storybook/addon-essentials should be version ^8.0.0', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
    const devDeps = packageJson.devDependencies;

    expect(devDeps['@storybook/addon-essentials']).toBe('^8.0.0');
  });

  it('@storybook/addon-interactions should be version ^8.0.0', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
    const devDeps = packageJson.devDependencies;

    expect(devDeps['@storybook/addon-interactions']).toBe('^8.0.0');
  });

  it('@storybook/react-vite should be version ^8.0.0', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
    const devDeps = packageJson.devDependencies;

    expect(devDeps['@storybook/react-vite']).toBe('^8.0.0');
  });

  it('storybook core should be version ^8.0.0', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
    const devDeps = packageJson.devDependencies;

    expect(devDeps['storybook']).toBe('^8.0.0');
  });

  it('all storybook packages should have matching major versions', () => {
    const packageJsonPath = path.join(__dirname, '..', 'package.json');
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
    const devDeps = packageJson.devDependencies;

    const storybookPackages = Object.entries(devDeps)
      .filter(([name]) => name.startsWith('@storybook/') || name === 'storybook')
      .map(([name, version]) => ({ name, version: String(version) }));

    // Extract major version from each package (e.g., "^8.0.0" -> 8)
    const majorVersions = storybookPackages.map((pkg) => {
      const match = pkg.version.match(/\^?(\d+)/);
      return match ? parseInt(match[1], 10) : null;
    });

    const firstVersion = majorVersions[0];
    expect(majorVersions.every((v) => v === firstVersion)).toBe(true);
  });
});
