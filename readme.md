# trnsys-odbc

Store TrnSys Simulation Results in MS Excel, MS Access, SQLite and more ODBC Databases.

## Supported Databases

### Tested

- Microsoft Access
- Microsoft Excel
- SQLite

### Theoretical Support


- Microsoft SQL Server
- All other ODBC-compliant databases

## Usage

### Pre-requisites

#### Install Drivers
Install the ODBC driver for the database you want to connect to.

- If you've installed Microsoft Office, then you already have the drivers for Microsoft Access and Excel.
- For SQLite, you can download the driver from [here](https://www.ch-werner.de/sqliteodbc/).

#### Install the component
- Copy the dll from release to your TrnSys User Dll Folder, e.g. `C:\TRNSYS18\UserLib\ReleaseDLLs\`
- If you use Simulation Studio, copy the .tmf file to the Perfomas Folder, e.g. `C:\TRNSYS18\Studio\Proformas\`

### Parameters

| No | Name             | Description                                                                                                                                                                        | Default |
|----|------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------|
| 1  | `PrintInterval`  | Interval to print simulation results to the database.                                                                                                                              | 1       |
| 2  | `DriverMode`     | Integer between 1 and 4. Driver Mode determines how to write the data to the database. <br> MsAccessFile = 1, <br> MsExcelFile = 2, <br> SqliteFile = 3, <br> ODBC Connection String = 4 | 1       |
| 3  | `NumberOfInputs` | Number of inputs connected to this component.                                                                                                                                      | 3       |

### Special Cards / Labels

All the answers to the cards should be wrapped in double quotes. For example, if the answer is `My Database`, then it
should be written as `"My Database"`.

| No | Name                | Description                                                                                                                                                                            |
|----|---------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1  | `Labels`            | Please do not edit this. This is used to store the number of labels.                                                                                                                   |
| 2  | `Connection String` | If `DriverMode` is 4, then this card is used to store the connection string. If the file-based database is used (mode 1-3), then the connection string should be the path to the file. |
| 3  | `Table Name`        | Name of the table to write the data.                                                                                                                                                   |
| 4  | `Variant Name`      | Name of the variant to write the data. At the beginning of the simulation, **all data** with the same variant name will be **deleted** from the table.                                 |
| 5+ | `Input Names`       | The name of all columns, one by one, each wrapped by double quotation marks.                                                                                                           |

## Example Deck File

```text
*------------------------------------------------------------------------------


* Model "Component1" (Type 256)
* 

UNIT 3 TYPE 256	 Component1

PARAMETERS 3
STEP		! 1 PrintInterval
2		! 2 DriverMode (Excel File)
3		! 3 NumberOfInputs

INPUTS 3
0,0		! [unconnected]
0,0		! [unconnected]
0,0		! [unconnected]
*** INITIAL INPUT VALUES
0 0 0

*** 6 Labels (3 fixed + 3 inputs)
LABELS 6
"result.xlsx" ! Connection String (path to the file relative to the simulation folder)
"SimulationResult" ! Table Name
"Variant1" ! Variant Name
*** Column Names
col1 "some col2" "another col3"
*------------------------------------------------------------------------------
```

## FAQ

#### Where can I find the connection string?
It's driver-dependent. See [this website](https://www.connectionstrings.com/) for more details.

## Debugging

Logs are written to the file `type_error.log` in the same directory as the simulation file if simulation terminated with
error. Also, warnings and errors are stored in TrnSys log file.
