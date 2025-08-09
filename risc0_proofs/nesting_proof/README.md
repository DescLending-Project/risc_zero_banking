# Nesting Proof (Deprecated)

⚠️ **This implementation is deprecated.** Please head over to `../../score_publisher/` for the current working release of the nesting proof functionality.

## Overview

This directory contains legacy nesting proof logic that has been superseded by the integrated implementation in the `score_publisher` component. While the nesting logic itself is no longer maintained, this repository still provides utilities for extracting image IDs from binary proof receipts.

## Current Functionality

### Image ID Extraction

You can still use this repository to extract image IDs from RISC0 binary proof receipts, which is useful for debugging and proof analysis.

## Building

```bash
RISC0_USE_DOCKER=1 cargo build --release
```
## Usage
# Extract Image ID from Receipt
To extract the image ID from a binary receipt file:
```bash
bashcargo run -p host --bin extract_image_id --release -- receipts/rece
```
