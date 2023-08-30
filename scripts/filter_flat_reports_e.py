# This script is intended to filter all reports so that they only contain water
# right usage locations where the legal department is 'E'.
# Then it also trims it down to remove obsolete columns.

import pandas as pd
import os

data_dir = os.path.dirname(__file__) + "/../data"

# Load CSV file into a DataFrame
df = pd.read_csv(data_dir + "/reports.csv", sep=";", dtype=str)

# Filter the DataFrame to keep only rows where 'Abteilungskürzel' or 'legal
# department abbreviation' is 'E'
filtered_df = df[
    (df.get("Abteilungskürzel") == "E") |
    (df.get("legal department abbreviation") == "E")
]

# Remove columns with all values empty or NaN
cleaned_df = filtered_df.dropna(axis=1, how="all")

# Save the cleaned DataFrame to a new CSV file
cleaned_df.to_csv(data_dir + "/reports_e.csv", sep=";", index=False)
