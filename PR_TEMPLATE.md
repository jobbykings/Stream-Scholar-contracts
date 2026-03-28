# Course Metadata Standard Implementation

## Summary
This PR implements a comprehensive JSON schema standard for course metadata to be stored on IPFS, along with pipeline improvements and documentation.

## Changes Made

### 📋 New Features
- **Course Metadata Schema** (`docs/course-metadata-standard.json`)
  - Complete JSON Schema Draft 7 compliant schema
  - Defines standard structure for course descriptions, thumbnails, and durations
  - Includes validation patterns for IPFS CIDs, wallet addresses, and data formats
  - Supports instructor information, pricing, categories, and learning objectives

- **Implementation Guide** (`docs/course-metadata-implementation-guide.md`)
  - Comprehensive documentation with usage examples
  - Integration guidelines for smart contracts and frontend
  - Best practices and migration instructions
  - IPFS integration patterns

### 🔧 Pipeline Improvements
- **Enhanced CI/CD Pipeline** (`.github/workflows/pipeline.yml`)
  - Added Node.js setup for JSON schema validation
  - Implemented JSON schema validation step
  - Added cargo dependency caching for faster builds
  - Improved error handling and emoji indicators
  - Added proper validation for course metadata schema

### 📦 Documentation
- Added `docs/package.json` for schema validation tools
- Detailed examples and validation patterns
- Integration examples for smart contracts

## Schema Features

### Core Fields
- `courseId`: Unique identifier with validation
- `title`, `description`: Course information with length limits
- `instructor`: Complete instructor object with wallet address validation
- `duration`: Duration in minutes with calculated hours
- `thumbnail`: IPFS CID with MIME type and metadata
- `createdAt/updatedAt`: ISO 8601 timestamps

### Optional Fields
- `category`: Primary/secondary categories and tags
- `difficulty`: beginner to expert levels
- `language`: ISO 639-1 language codes
- `price`: Multi-currency pricing support (XLM, USDC, ETH)
- `prerequisites`: Course dependency management
- `learningObjectives`: Structured learning outcomes
- `version`: Schema versioning
- `status`: Publication status tracking

## Validation
- JSON Schema Draft 7 compliant
- IPFS CID validation (v0 and v1)
- Stellar wallet address validation
- Comprehensive data type and format validation
- Size limits and pattern matching

## Testing
- Schema self-validation in CI/CD pipeline
- Example validation included
- Error handling for missing or invalid schemas

## Benefits
1. **Standardization**: Consistent metadata format across the platform
2. **Validation**: Built-in validation ensures data quality
3. **IPFS Integration**: Optimized for decentralized storage
4. **Developer Experience**: Clear documentation and examples
5. **Pipeline Quality**: Automated validation in CI/CD

## Files Changed
- `docs/course-metadata-standard.json` (new)
- `docs/course-metadata-implementation-guide.md` (new)
- `docs/package.json` (new)
- `.github/workflows/pipeline.yml` (updated)

## Labels
- documentation
- frontend
- enhancement
- ci/cd

## Related Issues
- Implements Course_Metadata_Standard feature request
- Addresses pipeline validation requirements
- Provides foundation for frontend integration

## Testing Instructions
1. Run `npm install` in docs directory
2. Run `npm run validate` to test schema validation
3. Check CI/CD pipeline for schema validation step
4. Review implementation examples in the guide

## Checklist
- [x] Schema is JSON Schema Draft 7 compliant
- [x] All examples validate against the schema
- [x] CI/CD pipeline includes schema validation
- [x] Documentation is comprehensive and clear
- [x] IPFS CID patterns are correct
- [x] Stellar address validation is implemented
- [x] Pipeline improvements are tested
