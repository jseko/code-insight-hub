"""
Simple converter: reads the CSV and writes an Excel file.
Run:
  pip install pandas openpyxl
  python outputs/convert_csv_to_xlsx.py
This creates `rd_schedule_2025_Jul_Dec.xlsx` next to the CSV.
"""
import pandas as pd
from pathlib import Path

csv_path = Path(__file__).parent / "rd_schedule_2025_Jul_Dec.csv"
xlsx_path = csv_path.with_suffix('.xlsx')

df = pd.read_csv(csv_path)
df.to_excel(xlsx_path, index=False)
print(f"Wrote {xlsx_path}")
