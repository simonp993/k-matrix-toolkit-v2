import concurrent.futures
import glob
import warnings
from itertools import repeat

import numpy as np
import pandas as pd

pd.set_option("display.max_columns", None)


def get_file_path(directory_path: str):
    """
    Creates a list of all paths, which lead to all the .xlsx files in the given directory
    :param directory_path: String keeping the name of the NIP directory
    :return temp_file_path: List containing all the paths to the files
    """
    file_dir = directory_path + "/"
    list(glob.glob(file_dir + "/**/*.xlsx", recursive=True))
    file_paths = glob.glob(file_dir + "/**/*.xlsx", recursive=True)
    temp_file_path = []
    for filename in file_paths:
        if "KMatrix".lower() in filename.lower():
            temp_file_path.append(filename)
        elif "Vergleich".lower() in filename.lower():
            continue
        elif "RT".lower() in filename.lower():
            continue
        elif "NPM".lower() in filename.lower():
            continue
        else:
            continue

    return temp_file_path


# Alle Tabellen öffnen und als Dataframes abspeichern
def get_files(file_path: str):
    """
    Saves the .xlsx files (KMatrices) as Pandas Dataframes
    :param file_path: String containing the path to a .xlsx file
    :return df_kmatrix: Pandas Dataframe containing the data of the K-Matrix
    :return list_file_path_split: List conatining all Elements of the directory leading to the File
    """
    with warnings.catch_warnings(record=True):
        warnings.simplefilter("always")
        list_file_path_split = file_path.split("\\")
        file_name = list_file_path_split[-1]

        # Get the name of the bus to look in the needed sheet of .xlsx File, a lot of special cases needed to be determined cause lack of consistency in sheet naming
        if "MLBevo" in file_name:
            if "Fx" in file_name:
                bus_name = "MLBevo_FlexRay"
            else:
                file_name = file_name.split("_")
                index = file_name.index("KMatrix")
                if (file_name[index - 2] == "Konzern") or (
                    file_name[index - 2] == "MLBevo"
                ):
                    bus_name = file_name[index - 1]
                else:
                    bus_name = file_name[index - 2] + "_" + file_name[index - 1]

        else:
            file_name = file_name.split("_")
            try:
                index = file_name.index("KMatrix")
                if index > 1:
                    if "Premium" in file_name:
                        if file_name[index - 2] == "Premium":
                            bus_name = file_name[file_name.index("Premium") + 1]
                        else:
                            bus_name = (
                                file_name[file_name.index("Premium") + 1]
                                + "_"
                                + file_name[file_name.index("Premium") + 2]
                            )
                    else:
                        bus_name = (
                            file_name[file_name.index("KMatrix") - 2]
                            + "_"
                            + file_name[file_name.index("KMatrix") - 1]
                        )
                else:
                    bus_name = file_name[index - 1]
            except ValueError:
                return None, None, None

        if "Batterie" in bus_name:
            bus_name = bus_name.replace("Batterie_SUB", "B")

        # Load K-Matrix to pandas dataframe
        df_kmatrix = pd.ExcelFile(file_path)
        for item in df_kmatrix.sheet_names:
            if bus_name in item:
                df_kmatrix = pd.read_excel(file_path, item, header=0)
                break

    # Checks if all went right
    if not isinstance(df_kmatrix, pd.DataFrame):
        for sheet in range(10):
            df_kmatrix = pd.read_excel(file_path, sheet, header=0)
            if (
                ("Botschaften".lower() in df_kmatrix.columns.str.lower())
                and ("Signale".lower() in df_kmatrix.columns.str.lower())
                and ("Wertebereich".lower() in df_kmatrix.columns.str.lower())
            ):
                break
            else:
                continue

    # Add source_file to dataframe
    # df_kmatrix["source_file"] = file_path.split("/")[-1]

    return df_kmatrix, list_file_path_split, bus_name


def get_number_of_KMatrix(list_file_path_split: list, selected_kmatrix: str):
    """
    Get ID of chosen KMatrix
    Needed cause of same ID for different signals
    :param list_file_path_split: List of Lists containing all Elements of the directory leading to the File
    :param selected_kmatrix: String conataining name of selected KMatrix
    :return id_kmatrix: Number of Kmatrix in List of paths
    :return list_file_path_split: List of Lists containing all Elements of the directory leading to the File
    """
    id_kmatrix = 0
    for idx, file_path in enumerate(list_file_path_split):
        for item in file_path:
            if selected_kmatrix in item:
                id_kmatrix = idx
                break
    return id_kmatrix, list_file_path_split


def row_to_header(list_df_kmatrix: list):
    """
    Turns first row of Dataframe as its header
    :param list_df_kmatrix: List of all Kmatrices as Dataframes
    :return list_df_kmatrix: Adapted list
    """
    for idx, df_kmatrix in enumerate(list_df_kmatrix):
        df_kmatrix.columns = df_kmatrix.iloc[0]
        list_df_kmatrix[idx] = list_df_kmatrix[idx].drop(list_df_kmatrix[idx].index[0])
    return list_df_kmatrix


def get_signal_name(
    signal_id: str, list_df_kmatrix: list, list_kmatrix_paths: list, id_kmatrix: int
):
    """
    Get name of Signal based on its ID
    :param signal_id: String containing the ID of the Signal
    :param list_df_kmatrix: List of all Kmatrices as Dataframes
    :param list_kmatrix_paths: List containing the Paths to the KMatrix files
    :param id_kmatrix: Number of Kmatrix where to look for name
    :return signal_name: Name of Signal as String
    """
    if "VLAN" not in list_kmatrix_paths[id_kmatrix]:
        temp_df_kmatrix = list_df_kmatrix[id_kmatrix][
            list_df_kmatrix[id_kmatrix]["Identifier [hex]"]
            .astype(str)
            .str.contains(signal_id)
        ]
        signal_name = temp_df_kmatrix.iloc[0, 0]
    else:
        temp_df_kmatrix = list_df_kmatrix[id_kmatrix][
            list_df_kmatrix[id_kmatrix]["PDU-ID [hex]"]
            .astype(str)
            .str.contains(signal_id)
        ]
        signal_name = temp_df_kmatrix.iloc[0, 14]

    return signal_name


def get_signal_id(
    signal_name: str, list_df_kmatrix: list, list_kmatrix_paths: list, id_kmatrix: int
):
    """
    Get Id of Signal based on its Name
    :param signal_name: String containing the Name of the Signal
    :param list_df_kmatrix: List of all Kmatrices as Dataframes
    :param list_kmatrix_paths: List containing the Paths to the KMatrix files
    :param id_kmatrix: Number of Kmatrix where to look for name
    :return signal_id: Name of Signal as String
    """
    if "FlexRay" in list_kmatrix_paths[id_kmatrix]:
        signal_id = ""
    elif "VLAN" not in list_kmatrix_paths[id_kmatrix]:
        temp_df_kmatrix = list_df_kmatrix[id_kmatrix][
            list_df_kmatrix[id_kmatrix]["Botschaft"]
            .astype(str)
            .str.contains(signal_name)
        ]
        signal_id = temp_df_kmatrix.iloc[0, 1]
    else:
        temp_df_kmatrix = list_df_kmatrix[id_kmatrix][
            list_df_kmatrix[id_kmatrix]["PDU"].astype(str).str.cohntains(signal_name)
        ]
        signal_id = temp_df_kmatrix.iloc[0, 16]

    return signal_id


def find_ecus_and_source_bus(
    list_df_kmatrix: list,
    list_file_path_split: list,
    signal_name: str,
    list_kmatrix_paths: list,
    list_all_bus: list,
):
    """
    Search all tables by the name of the signal and save to which ECUs it is sent and to which it is only routed
    :param list_df_kmatrix: List of all Kmatrices as Dataframes
    :param list_file_path_split: List of Lists containing all Elements of the directory leading to the File
    :param signal_name: String containing the Name of the Signal
    :param list_kmatrix_paths: List containing the Paths to the KMatrix files
    :param list_all_bus: List containing all the bus Names
    :return list_routing_ecus: List containing names of the routing ecus
    :return list_source_ecus: List containing names of the source ecus
    :return list_source_bus: List containing source buses from paths
    """
    list_routing_ecus, list_source_ecus, list_source_bus = [], [], []
    for idx, df_kmatrix in enumerate(list_df_kmatrix):
        try:
            # Remove all lines that do not contain the name of the signal
            temp_df_kmatrix = df_kmatrix[
                df_kmatrix["Botschaft"].astype(str).str.contains(signal_name)
            ]
            columns = temp_df_kmatrix.columns.values.tolist()
            count_rows, count_columns = temp_df_kmatrix.shape
            # Check in which columns it is specified whether the signal is sent or routed and save from which ecu it is saved or routed.
            if temp_df_kmatrix.empty is False:
                for y in range(count_columns):
                    for x in range(count_rows):
                        if "S*" == temp_df_kmatrix.iloc[x, y]:
                            list_routing_ecus.append(columns[y])
                        elif "S" == temp_df_kmatrix.iloc[x, y]:
                            list_source_ecus.append(columns[y])
                            list_source_bus.append(list_all_bus[idx])

        # The same as above, only for Ethernet or FlexRay buses
        except KeyError:
            kmatrix_header = df_kmatrix.iloc[0]
            temp_df_kmatrix = df_kmatrix[
                df_kmatrix["PDU"].astype(str).str.contains(signal_name)
            ]
            columns = temp_df_kmatrix.columns.values.tolist()
            count_rows, count_columns = temp_df_kmatrix.shape
            if temp_df_kmatrix.empty is False:
                if "VLAN" in list_kmatrix_paths[idx]:
                    for y in range(count_columns):
                        for x in range(count_rows):
                            if "S*" == temp_df_kmatrix.iloc[x, y]:
                                list_routing_ecus.append(kmatrix_header.iloc[y])
                            elif "S" == temp_df_kmatrix.iloc[x, y]:
                                list_source_ecus.append(kmatrix_header.iloc[y])
                                list_source_bus.append(list_all_bus[idx])
                else:
                    for y in range(count_columns):
                        for x in range(count_rows):
                            if "S*" == temp_df_kmatrix.iloc[x, y]:
                                list_routing_ecus.append(columns[y])
                            elif "S" == temp_df_kmatrix.iloc[x, y]:
                                list_source_ecus.append(columns[y])
                                list_source_bus.append(list_all_bus[idx])

    # Sort lists, convert to strings and remove unnecessary characters
    routing_ecus = (
        (str(list((set(list_routing_ecus)))))
        .replace("[", "")
        .replace("]", "")
        .replace("'", "")
    )

    source_ecus = (
        (str(list((set(list_source_ecus)))))
        .replace("[", "")
        .replace("]", "")
        .replace("'", "")
    )

    source_buses = (
        (str(list((set(list_source_bus)))))
        .replace("[", "")
        .replace("]", "")
        .replace("'", "")
    )

    return routing_ecus, source_ecus, source_buses


# Function not used anymore, but if there are bugs in the shown source bus, this may help
# def get_source_bus(list_source_bus_path):
#     """
#     Get name of source bus from file path
#     :param list_source_bus_path: List containing source buses from paths
#     :param list_source_bus_name: List containing names of the source buses
#     """
#     seperator = "_"
#     list_source_bus_name = []
#     for item in list_source_bus_path:
#         if "E3" in item:
#             inName = False
#             item = item.split(seperator)
#             temp = 1000
#             temp2 = []
#             for idx, item2 in enumerate(item):
#                 if "KMatrix" in item2 and inName:
#                     break
#                 elif "Premium" in item2:
#                     inName = True
#                     temp = idx
#                 elif idx > temp:
#                     temp2.append(item2)
#             temp2 = seperator.join(temp2)
#             list_source_bus_name.append(temp2)
#         elif "MLBevo" in item:
#             inName = False
#             item = item.split(seperator)
#             temp = 1000
#             temp2 = []
#             for idx, item2 in enumerate(item):
#                 if "KMatrix" in item2 and inName:
#                     break
#                 elif "Gen2" in item2:
#                     inName = True
#                     temp = idx
#                 elif idx > temp:
#                     temp2.append(item2)
#             temp2 = seperator.join(temp2)
#             list_source_bus_name.append(temp2)

#     list_source_bus_name = (
#         (str(list((set(list_source_bus_name)))))
#         .replace("[", "")
#         .replace("]", "")
#         .replace("'", "")
#     )

#     return list_source_bus_name


def get_result_of_one_df(
    df_kmatrix: pd.DataFrame,
    kmatrix_path: str,
    keyword: str,
    is_free_text_search: bool,
    is_case_sensitive: bool,
):
    """
    Search a kMatrix for the keyword
    :param df_kmatrix: Pandas Dataframe containing data from KMatrix
    :param kmatrix_path: String containing path to kMatrix
    :param keyword: String that represents the Keyword to search for
    :param is_free_text_search: Boolean deciding wheter or not Free text search should be executed
    :param is_case_sensitive: Boolean deciding wheter or not case sensitivity should be executed
    :return df_shown_kmatrix: Pandas Datframe that contains only the rows and columns in which a matching value was found
    :return kmatrix_path: String to later link the Table, so the user can open it
    """

    df_kmatrix.dropna(axis=1, how="all", inplace=True)
    df_kmatrix.dropna(axis=0, how="all", inplace=True)
    df_kMatrix_str = df_kmatrix.astype(str)

    displayed_columns_id = []
    columns = df_kMatrix_str.columns.values.tolist()

    # Extract the columns that should be displayed in the final table
    # and except if columns don't ecist
    displayed_columns_id.append(columns.index("Signale"))
    try:
        displayed_columns_id.append(columns.index("Signalkommentar"))
    except ValueError:
        df_shown_kmatrix = pd.DataFrame()
        kmatrix_path = None
        return df_shown_kmatrix, kmatrix_path

    # get the Id of the columns in Dtaframe, which should later be displayed whil eworking with the tool
    # Also rename the original columns to fit their display, this is needed cause of dump excel formatation
    for idx, column in enumerate(columns):
        if df_kMatrix_str.iloc[0, idx] == "Identifier [hex]":
            displayed_columns_id.append(idx)
            df_kmatrix.rename(columns={column: "Identifier [hex]"}, inplace=True)
        elif df_kMatrix_str.iloc[0, idx] == "InitWert roh [dez]":
            displayed_columns_id.append(idx)
            df_kmatrix.rename(columns={column: "InitWert roh [dez]"}, inplace=True)
        elif df_kMatrix_str.iloc[0, idx] == "FehlerWert roh [dez]":
            displayed_columns_id.append(idx)
            df_kmatrix.rename(columns={column: "FehlerWert roh [dez]"}, inplace=True)
        elif df_kMatrix_str.iloc[0, idx] == "PDU-ID [hex]":
            displayed_columns_id.append(idx)
            df_kmatrix.rename(columns={column: "Identifier [hex]"}, inplace=True)
        elif df_kMatrix_str.iloc[0, idx] == "Physikalische Werte":
            for idx2 in range(8):
                displayed_columns_id.append(idx + idx2)
                df_kmatrix.rename(
                    columns={columns[idx + idx2]: df_kMatrix_str.iloc[1, idx + idx2]},
                    inplace=True,
                )
        elif column == "Sender - Empfänger":
            for idx2 in range(
                columns.index("Signalkommentar")
                - columns.index("Sender - Empfänger")
                - 3
            ):
                displayed_columns_id.append(idx + idx2)
                if df_kMatrix_str.iloc[0, idx + idx2] != "nan":
                    df_kmatrix.rename(
                        columns={
                            columns[idx + idx2]: "Sender - Empfänger: "
                            + df_kMatrix_str.iloc[0, idx + idx2]
                        },
                        inplace=True,
                    )
                else:
                    df_kmatrix.rename(
                        columns={
                            columns[idx + idx2]: "Sender - Empfänger: "
                            + df_kMatrix_str.iloc[1, idx + idx2]
                        },
                        inplace=True,
                    )
    displayed_columns_id = list(displayed_columns_id)
    displayed_columns_id.sort()

    # Search for the keyword, for a completely free text search, a list comprehension method is the fastest,
    # if it is not a free text search using numpy arrays.where function is the fastest
    # Get all matching results in current Datframe in list_master_index_of_keyword
    list_index_of_keyword = []
    if is_free_text_search:
        if is_case_sensitive:
            keyword = keyword.lower()
            for column in df_kMatrix_str.columns:
                list_index_of_keyword.append(
                    [
                        i
                        for i, value in enumerate(df_kMatrix_str[column].str.lower())
                        if keyword in value
                    ]
                )
        else:
            for column in df_kMatrix_str.columns:
                list_index_of_keyword.append(
                    [
                        i
                        for i, value in enumerate(df_kMatrix_str[column])
                        if keyword in value
                    ]
                )
        list_master_index_of_keyword = []
        for index in list_index_of_keyword:
            if len(index) != 0:
                list_master_index_of_keyword.extend(list(index))
        list_master_index_of_keyword.sort()
        list_master_index_of_keyword = list(dict.fromkeys(list_master_index_of_keyword))
    else:
        if is_case_sensitive:
            keyword = keyword.lower()
            for column in df_kMatrix_str.columns:
                np_kmatrix = df_kMatrix_str[column].str.lower().values
                list_index_of_keyword.append(np.where(np_kmatrix == keyword))
        else:
            for column in df_kMatrix_str.columns:
                np_kmatrix = df_kMatrix_str[column].values
                list_index_of_keyword.append(np.where(np_kmatrix == keyword))
        list_master_index_of_keyword = []
        for index in list_index_of_keyword:
            if len(index[0]) != 0:
                list_master_index_of_keyword.extend(list(index[0]))
        list_master_index_of_keyword.sort()
        list_master_index_of_keyword = list(dict.fromkeys(list_master_index_of_keyword))

    # Only use rows of Datframe where matching value was found and only the columns, which should be displayed
    df_kmatrix = df_kmatrix.iloc[list_master_index_of_keyword]
    df_shown_kmatrix = df_kmatrix.iloc[:, displayed_columns_id].copy()
    df_shown_kmatrix.dropna(axis=1, how="all", inplace=True)

    # If no results were found, lose the path
    if df_shown_kmatrix.empty:
        kmatrix_path = None
    else:
        df_shown_kmatrix["source_file"] = str(kmatrix_path.split("/")[-1])

    return df_shown_kmatrix, kmatrix_path


def get_results_of_all_dfs(
    list_df_kmatrix: list,
    list_kmatrix_paths: list,
    keyword: str,
    is_free_text_search: bool,
    is_case_sensitive: bool,
    max_workers: int,
):
    """
    Search all KMatrices for keyword using multiprocessing and merge the results
    :param list_df_kmatrix: List of all Kmatrices as Dataframes
    :param list_kmatrix_paths: List containing the Paths to the KMatrix files
    :param keyword: String that represents the Keyword to search for
    :param is_free_text_search: Boolean deciding wheter or not Free text search should be executed
    :param is_case_sensitive: Boolean deciding wheter or not case sensitivity should be executed
    :return df_final_kmatrix_result: Pandas Datframe conataining all the data matching the keyword
    :return list_kmatrix_paths: List containing the paths to excel Files where keyword was found
    """

    # extract kmatrix names from paths
    list_kmatrix_names = []
    for kmatrix_path in list_kmatrix_paths:
        list_kmatrix_names.append(kmatrix_path.split("/")[-1])

    with concurrent.futures.ProcessPoolExecutor(max_workers=max_workers) as executor:
        results = executor.map(
            get_result_of_one_df,
            list_df_kmatrix,
            list_kmatrix_paths,
            repeat(keyword),
            repeat(is_free_text_search),
            repeat(is_case_sensitive),
        )
        results = list(results)

    list_df_shown_kmatrix = []
    list_kmatrix_paths = []

    # get results from multiprocessing
    for result in results:
        list_df_shown_kmatrix.append(result[0])
        list_kmatrix_paths.append(result[1])

    # If Kmatrix is empty(keyword was not found) delete it
    list_empty_kmatrix_id = []
    for i, df_kmatrix in enumerate(list_df_shown_kmatrix):
        if df_kmatrix.empty:
            list_empty_kmatrix_id.append(i)
    for i in sorted(list_empty_kmatrix_id, reverse=True):
        del list_df_shown_kmatrix[i]

    print("#" * 100)
    print(len(list_df_shown_kmatrix))

    columns_of_table = [
        "Signale",
        "Beschreibung",
        "Signalkommentar",
        "source_file",
        "Identifier [hex]",
        "InitWert roh [dez]",
        "FehlerWert roh [dez]",
        "Min Rohwert [dez]",
        "Max Rohwert [dez]",
        "phy Werte [dez]",
        "Einheit",
        "Offset",
        "Skalierung",
        "Rohwert [dez]",
    ]
    # Merge all dataframes where keywords were found into one
    if len(list_df_shown_kmatrix) > 1:

        # Start with the first DataFrame in the list and reduce
        # columns for merge
        df_final_kmatrix_result = list_df_shown_kmatrix[0][
            list_df_shown_kmatrix[0].columns[
                list_df_shown_kmatrix[0].columns.isin(columns_of_table)
            ]
        ]

        # Merge subsequent DataFrames in the list
        for df_kmatrix in list_df_shown_kmatrix[1:]:
            # reduce columns before merge
            df_kmatrix = df_kmatrix[
                df_kmatrix.columns[df_kmatrix.columns.isin(columns_of_table)]
            ]
            df_final_kmatrix_result = pd.merge(
                df_final_kmatrix_result, df_kmatrix, how="outer"
            )
    else:
        df_final_kmatrix_result = list_df_shown_kmatrix[0]

    # Create temporary so even if no results were found, there will be no error showing them
    # Also fills 'nan' as string to column if empty, so pyqt can show it
    temp_df = pd.DataFrame()
    for column in columns_of_table:
        try:
            column_values = df_final_kmatrix_result[column].tolist()
            del df_final_kmatrix_result[column]
            temp_df[column] = column_values
        except KeyError:
            temp_df[column] = pd.Series(
                [np.nan for x in range(len(temp_df.index))], index=temp_df.index
            )
    df_final_kmatrix_result = temp_df

    # delete rows with void in Signale
    df_final_kmatrix_result = df_final_kmatrix_result[
        ~df_final_kmatrix_result["Signale"].str.contains("void", na=False)
    ]

    # delete rows with all nan values in Signale
    df_final_kmatrix_result.dropna(subset=["Signale"], inplace=True)

    # delete full duplicates
    df_final_kmatrix_result.drop_duplicates(inplace=True, ignore_index=True)

    # create metadata dataframe for excel export with all kmatrix names
    df_kmatrix_names = pd.DataFrame(
        list_kmatrix_names, columns=["searched_kmatrix_names"]
    )

    # create metadata dataframe for excel export with all user inputs
    df_user_inputs = pd.DataFrame(
        {
            "keyword": [keyword],
            "is_free_text_search": [is_free_text_search],
            "is_case_sensitive": [is_case_sensitive],
        }
    )

    list_kmatrix_paths = [x for x in list_kmatrix_paths if x is not None]

    return df_final_kmatrix_result, df_kmatrix_names, df_user_inputs
