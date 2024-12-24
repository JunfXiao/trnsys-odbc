# trnsys-odbc

Store TrnSys Simulation Results in MS Excel, SQLite and more ODBC Databases.

## Supported Databases

### Tested

- Microsoft Access
- Microsoft Excel

### Theoretical Support

- SQLite
- Microsoft SQL Server
- All other ODBC compliant databases

## Usage

### Pre-requisites

Install the ODBC driver for the database you want to connect to.

- If you've installed Microsoft Office, then you already have the drivers for Microsoft Access and Excel.
- For SQLite, you can download the driver from [here](https://www.ch-werner.de/sqliteodbc/).

### Parameters

| No | Name             | Description                                                                                                                                                                        | Default |
|----|------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------|
| 1  | `PrintInterval`  | Interval to print simulation results to the database.                                                                                                                              | 1       |
| 2  | `DriverMode`     | Integer between 1 and 4. Driver Mode determines how to write the data to the database. <br> MsAccessFile = 1, <br> MsExcelFile = 2, <br> SqliteFile = 3, <br> ConnectionString = 4 | 1       |
| 3  | `NumberOfInputs` | Number of inputs connected to this component.                                                                                                                                      | 3       |

### Special Cards / Labels

All the answers of the cards should be wrapped in double quotes. For example, if the answer is `My Database`, then it
should be written as `"My Database"`.

| No | Name               | Description                                                                                                                                                                                 |
|----|--------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1  | `Labels`           | Please do not edit this. This is used to store the number of labels.                                                                                                                        |
| 2  | `ConnectionString` | If `DriverMode` is 4, then this card is used to store the connection string. If file-based database is used (mode 1-3), then the connection string should be the path to the file.          |
| 3  | `TableName`        | Name of the table to write the data.                                                                                                                                                        |
| 4  | `Input Names`      | Names of the inputs connected to this component, separated by comma. The number of labels should be equal to the number of inputs connected to this component. Example: `"IN1, IN 2, IN_3"` |
| 5  | `Variant Name`     | Name of the variant to write the data. At the beginning of the simulation, **all data** with the same variant name will be **deleted** from the table.                                      |

## Example Deck File

```text
*------------------------------------------------------------------------------


* Model "Component1" (Type 256)
* 

UNIT 3 TYPE 256	 Component1

PARAMETERS 3
STEP		! 1 PrintInterval
2		! 2 DriverMode
3		! 3 NumberOfInputs

INPUTS 3
0,0		! [unconnected]
0,0		! [unconnected]
0,0		! [unconnected]
*** INITIAL INPUT VALUES
0 0 0 
LABELS 4
"result.xlsx"
"SimulationResult"
"col1, some col2, another col3"
"Variant1"
*------------------------------------------------------------------------------
```

## Debugging

Logs are written to the file `type_error.log` in the same directory as the simulation file if simulation terminated with
error. Also, warnings and errors are stored in TrnSys log file.
