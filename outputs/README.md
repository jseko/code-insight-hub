Files created:
- rd_schedule_2025_Jul_Dec.csv  — final schedule and cost breakdown (Jul–Dec 2025)
- convert_csv_to_xlsx.py      — optional script to convert the CSV to an Excel .xlsx file

How to open/use:
1) Open the CSV directly with Excel (Excel will parse it).
2) To generate a true .xlsx file, run:

```bash
pip install pandas openpyxl
python outputs/convert_csv_to_xlsx.py
```

This will create `outputs/rd_schedule_2025_Jul_Dec.xlsx`.
