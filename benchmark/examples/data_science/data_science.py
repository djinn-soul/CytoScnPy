"""
Data Science Example
Run with: cytoscnpy examples/data_science.py --danger
"""

import pandas as pd
import numpy as np

def process_data():
    df = pd.DataFrame({'A': [1, 2, 3], 'B': [4, 5, 6]})
    
    # Method chaining (should be handled correctly)
    result = (
        df.assign(C=lambda x: x.A + x.B)
          .drop(columns=['B'])
          .head(2)
    )
    print(result)

    # NumPy usage
    arr = np.array([1, 2, 3])
    print(arr.mean())

# CSP-D102: Potential SQL Injection in pandas
def unsafe_pandas_sql(conn, user_input):
    # This should trigger a warning if heuristics catch it
    query = f"SELECT * FROM users WHERE name = '{user_input}'"
    pd.read_sql(query, conn)


def main():
    process_data()

if __name__ == "__main__":
    main()
