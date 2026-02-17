import type { Config } from '@jest/types';

const config: Config.InitialOptions = {
  collectCoverage: true,
  collectCoverageFrom: ['<rootDir>/src/**/*.(t|j)s'],
  coverageDirectory: '<rootDir>/coverage',
  coverageReporters: ['html'],
  setupFiles: ['./jest.setup.ts'],
  testPathIgnorePatterns: ['<rootDir>/dist/'],
  roots: ['<rootDir>/src/', '<rootDir>/test/'],
  transformIgnorePatterns: ['/node_modules/(?!@mysten|@scure|@noble)/'],
  transform: {
    '^.+\\.m?[tj]sx?$': '@swc/jest',
  },
};

export default config;
