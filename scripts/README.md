# Scripts

Utility scripts for BridgeLab development and QA.

## `test_plan_to_excel.py`

Export the TEST_PLAN.md file to a formatted Excel workbook.

### Installation

```bash
pip install openpyxl
```

### Usage

```bash
# Default: reads TEST_PLAN.md, writes TEST_PLAN.xlsx
python scripts/test_plan_to_excel.py

# Custom input/output
python scripts/test_plan_to_excel.py -i docs/TEST_PLAN.md -o build/tests.xlsx
```

### Output structure

- **Index sheet**: overview of all test sections with counts
- **One sheet per section**: test cases with full formatting
- **Metadata sheet**: generation info, totals

### Features

- Priority cells colored (P0 red, P1 yellow, P2 green, P3 blue)
- Status cells colored (pass green, fail red, partial yellow)
- Frozen header row for easy scrolling
- Additional columns added for tracking: Tested By, Tested At, Notes
- Column widths tuned per column type
- Unicode status symbols recognized (\u2705 \u274c \u26a0 \u23f8 \u2753)

### Workflow

1. Update `TEST_PLAN.md` when new features are added
2. Run this script to regenerate `TEST_PLAN.xlsx`
3. Share `.xlsx` with the QA team
4. Testers fill Status and Notes columns
5. Optional: copy notable results back to markdown for history

The `.xlsx` file is git-ignored to avoid merge conflicts on binary file.
