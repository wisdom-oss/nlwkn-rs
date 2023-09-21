import os
import pandas as pd

DATA_DIR = os.path.dirname(__file__) + "/../data"

REPORTS_FILE = DATA_DIR + "/reports.csv"
FILTER_TABLE_FILE = DATA_DIR + "/20230920_Wasserrechte_lagebezogene_Auswahl.xlsx"
OUTPUT_FILE = DATA_DIR + "/20230920_Wasserrechte_lagebezogene_Auswahl_REPORTS.csv"

REPORTS_COLS = ('Wasserrecht Nr.', 'Nutzungsort Nr.')
FILTER_TABLE_COLS = ('Wasserrecht_Nr', 'Nutzungsort_Nr')

reports_df = pd.read_csv(REPORTS_FILE, sep=";", dtype=str)
reports_dict = {(row[REPORTS_COLS[0]], row[REPORTS_COLS[1]]): row for _, row in reports_df.iterrows()}

filter_table_df = pd.read_excel(FILTER_TABLE_FILE, dtype=str)

matched_rows = []
unmatched_count = 0
for _, row in filter_table_df.iterrows():
    key = (row[FILTER_TABLE_COLS[0]], row[FILTER_TABLE_COLS[1]])
    if key in reports_dict:
        matched_rows.append(reports_dict[key])
    else:
        unmatched_count += 1
        print(f"Warning: Entry {key} not found in reports.csv")

output_df = pd.DataFrame(matched_rows)
output_df = output_df.dropna(axis=1, how='all')
output_df.to_csv(OUTPUT_FILE, sep=";", index=False)

print(f"{unmatched_count} out of {len(filter_table_df)} entries could not be found in reports.csv.")
