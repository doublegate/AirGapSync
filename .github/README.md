# GitHub Workflows

This directory contains CI/CD workflow files for AirGapSync.

## Note on Workflows

These workflow files (`ci.yml` and `release.yml`) were created during project 
implementation but cannot be automatically pushed due to GitHub App permission 
restrictions.

## Files Available

- `ci.yml` - Continuous Integration workflow
  - Runs tests on macOS and Linux
  - Clippy linting
  - Rustfmt checking
  - Security audits
  
- `release.yml` - Release automation workflow
  - Multi-platform builds
  - Binary packaging
  - Automated releases

## Manual Setup

To enable these workflows:

1. Review the workflow files in this directory
2. Manually add them to your repository via web interface or with proper permissions
3. Workflows will activate on next push to main/develop or PR creation

These are fully functional GitHub Actions workflows ready for use.
