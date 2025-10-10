#language slang 2026

module matrix;

public extension float4x4 {
    // Extension function that provides a helper to return the column of a matrix, since slang's array accessor
    // is always selecting the row.
    [ForceInline]
    public func getColumn(column_index: uint) -> float4 {
        return float4(this[0][column_index], this[1][column_index], this[2][column_index], this[3][column_index]);
    }

    // Extension function that provides a helper to return the row of a matrix.
    [ForceInline]
    public func getRow(row_index: uint) -> float4 {
        return this[row_index];
    }
}
