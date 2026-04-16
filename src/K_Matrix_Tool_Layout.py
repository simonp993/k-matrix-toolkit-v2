import concurrent.futures
import datetime
import os
import sys
import time
import subprocess

import pandas as pd
from PyQt6 import QtCore, QtGui, QtWidgets
from PyQt6.QtCore import (
    QAbstractTableModel,
    QObject,
    QRunnable,
    Qt,
    QThreadPool,
    QUrl,
    pyqtSignal,
    pyqtSlot,
)
from PyQt6.QtGui import QMovie, QPixmap
from PyQt6.QtWidgets import (
    QFileDialog,
    QLabel,
    QListWidget,
    QListWidgetItem,
    QMainWindow,
    QMessageBox,
    QTableView,
    QLineEdit,
)

import src.K_Matrix_Tool_Functions as functions

basedir = os.path.dirname(__file__)


def open_excel_file_in_excel(file_path):
    """
    This function opens the specified file in Excel using the default application.
    Identifies the operating system and uses the appropriate command to open the file.

    :param file_path: _description_
    :type file_path: _type_
    """

    # Check if the file exists
    if not os.path.isfile(file_path):
        print(f"File not found at path: {file_path}")
        return

    # Determine the appropriate command to open the file based on the OS
    if os.name == "posix":  # macOS
        command = "open"
        # Construct the command to open the file with the default application
        try:
            subprocess.run([command, file_path], check=True)
            print(f"Opened {file_path} in Excel.")
        except subprocess.CalledProcessError as e:
            print(f"Error opening {file_path} in Excel: {e}")
    elif os.name == "nt":  # Windows
        # command = "start"
        file_path = file_path.replace("/", "\\")
        command = f"start \"excel\" \"{file_path}\""
        os.system(command)
    else:
        print("Unsupported operating system.")
        return



class PandasModel(QAbstractTableModel):
    """
    Class which builds a model for the QTableView from a dataframe
    (Found on https://learndataanalysis.org/display-pandas-dataframe-with-pyqt5-qtableview-widget/)
    """

    def __init__(self, data):
        QAbstractTableModel.__init__(self)
        self._data = data

    def rowCount(self, parent=None):
        """
        Counts rows of the data frame
        """
        return self._data.shape[0]

    def columnCount(self, parent=None):
        """
        Counts rows of the data frame
        """
        return self._data.shape[1]

    def data(self, index, role=Qt.ItemDataRole.DisplayRole):
        """
        Reads data from dataframe
        """
        if index.isValid():
            if role == Qt.ItemDataRole.DisplayRole:
                return str(self._data.iloc[index.row(), index.column()])
        return None

    def headerData(self, col, orientation, role):
        """
        Reads header data from dataframe
        """
        if (
            orientation == Qt.Orientation.Horizontal
            and role == Qt.ItemDataRole.DisplayRole
        ):
            return self._data.columns[col]
        return None


class WorkerSignals(QObject):
    """
    Class which contains signals that can be sent by the Worker class
    """

    # Resulting K-Matrices that will be searched
    results = pyqtSignal(list)
    # Reults of the search of the K-Matrices
    search_results = pyqtSignal(list)
    was_keyword_found = pyqtSignal(bool)


class Worker(QRunnable):
    """
    Class to start a new thread in which multiprocessing is used without freezing the UI
    """

    def __init__(
        self,
        list_kmatrix_paths: list,
        list_df_kmatrix: list,
        keyword: str,
        is_free_text_search: bool,
        is_case_sensitive: bool,
        is_start_clicked: str,
        max_executers: int,
    ):
        super(Worker, self).__init__()
        self.list_kmatrix_paths = list_kmatrix_paths
        self.list_df_kmatrix = list_df_kmatrix
        self.keyword = keyword
        self.is_free_text_search = is_free_text_search
        self.is_case_sensitive = is_case_sensitive
        self.is_start_clicked = is_start_clicked
        self.signals = WorkerSignals()
        self.max_executers = max_executers

    @pyqtSlot()
    def run(self):
        """
        If the start button in the UI is clicked start reading the data from the excel files to the Dataframes
        If the start button wasn't clicked but the Worker Object was called, it is to start searching for the keyword so this process will be started
        """
        if self.is_start_clicked:
            # Read data from KMatrices
            with concurrent.futures.ProcessPoolExecutor(
                max_workers=self.max_executers
            ) as executor:
                results = executor.map(functions.get_files, self.list_kmatrix_paths)
                print(results)
                results = list(results)

                # Remove any result which returned only None, because of empty KMatrices or similar
                results = [result for result in results if result[2] is not None]

                if not results:
                    # No matrices found
                    self.signals.results.emit([])
                else:
                    self.signals.results.emit(results)

        else:
            # Search for keyword in data
            try:
                (
                    df_final_kmatrix_result,
                    df_kmatrix_names,
                    df_user_inputs,
                ) = functions.get_results_of_all_dfs(
                    self.list_df_kmatrix,
                    self.list_kmatrix_paths,
                    self.keyword,
                    self.is_free_text_search,
                    self.is_case_sensitive,
                    self.max_executers,
                )

                # send the results to the UI from the workers
                search_results = [
                    df_final_kmatrix_result,
                    df_user_inputs,
                    df_kmatrix_names,
                ]
                self.signals.search_results.emit(search_results)

            except IndexError:
                # Nothing was found, so a Warning will popup
                was_keyword_found = True
                self.signals.was_keyword_found.emit(was_keyword_found)


class Ui_PDU_Search(QMainWindow):
    """
    Class that builds the gui and carries the functions of the gui
    Elements of GUI were build using qt Designer and than transformed to python code
    Afterwards the functionality of buttons etc. was added manually --> so for simple design widgets there will not be a lot of comments
    """

    def __init__(self):
        super(Ui_PDU_Search, self).__init__()
        self.left_right_top_bottom_margin = 30
        self.box_height = 40
        self.text_height = 30
        self.button_width = 120
        self.long_box_width = 600
        self.distance_between_boxes = 15
        self.distance_between_text_and_box = 8
        self.distance_between_box_and_text = 20
        self.first_box_top = 55
        self.distance_to_next_section = 25
        self.distance_to_next_paragraph = 40
        self.second_section_top = (
            self.left_right_top_bottom_margin
            + self.text_height
            + self.distance_between_text_and_box
            + self.box_height
            + self.distance_between_boxes
            + self.box_height
            + self.distance_to_next_section
        )
        self.third_section_top = (
            self.second_section_top
            + self.second_section_top  # boxes are exactly the same
        )
        self.mid_font_size = 12
        self.large_font_size = 14

    def setupUi(self, PDU_Search):
        """
        Create user interface
        """

        PDU_Search.setObjectName("PDU_Search")
        PDU_Search.setFixedSize(800, 900)

        self.threadpool = QThreadPool()

        self.centralwidget = QtWidgets.QWidget(PDU_Search)
        self.centralwidget.setObjectName("centralwidget")

        self.line = QtWidgets.QFrame(self.centralwidget)
        self.line.setGeometry(
            QtCore.QRect(
                -10,
                self.second_section_top,
                821,
                20,
            )
        )
        self.line.setFrameShadow(QtWidgets.QFrame.Shadow.Plain)
        self.line.setLineWidth(3)
        self.line.setFrameShape(QtWidgets.QFrame.Shape.HLine)
        self.line.setObjectName("line")

        self.line2 = QtWidgets.QFrame(self.centralwidget)
        self.line2.setGeometry(QtCore.QRect(-10, self.third_section_top, 821, 20))
        self.line2.setFrameShadow(QtWidgets.QFrame.Shadow.Plain)
        self.line2.setLineWidth(3)
        self.line2.setFrameShape(QtWidgets.QFrame.Shape.HLine)
        self.line2.setObjectName("line2")

        self.checkBoxFreeTextSearch = QtWidgets.QCheckBox(self.centralwidget)
        self.checkBoxFreeTextSearch.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.checkBoxFreeTextSearch.setFont(font)
        self.checkBoxFreeTextSearch.setObjectName("checkBoxFreeTextSearch")

        self.checkBoxCase = QtWidgets.QCheckBox(self.centralwidget)
        self.checkBoxCase.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.checkBoxCase.setFont(font)
        self.checkBoxCase.setObjectName("checkBoxCase")

        self.inputKeyword = QtWidgets.QLineEdit(self.centralwidget)
        self.inputKeyword.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.inputKeyword.setFont(font)
        self.inputKeyword.setObjectName("inputKeyword")

        self.findButton = QtWidgets.QPushButton(self.centralwidget)
        self.findButton.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.findButton.setFont(font)
        self.findButton.setCursor(Qt.CursorShape.PointingHandCursor)
        self.findButton.setObjectName("findButton")

        # self.resetButton = QtWidgets.QPushButton(self.centralwidget)
        # self.resetButton.setGeometry(QtCore.QRect(0, 0, 0, 0))
        # font = QtGui.QFont()
        # font.setPointSize(self.mid_font_size)
        # self.resetButton.setFont(font)
        # self.resetButton.setCursor(Qt.CursorShape.PointingHandCursor)
        # self.resetButton.setObjectName("resetButton")

        self.directory_path = QtWidgets.QLineEdit(self.centralwidget)
        self.directory_path.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box,
                self.long_box_width,
                self.box_height,
            )
        )
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.directory_path.setFont(font)
        self.directory_path.setObjectName("directory_path")

        self.searchButton = QtWidgets.QPushButton(self.centralwidget)
        self.searchButton.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin
                + self.long_box_width
                + self.distance_between_boxes,
                self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box,
                self.button_width,
                self.box_height,
            )
        )
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.searchButton.setFont(font)
        self.searchButton.setCursor(Qt.CursorShape.PointingHandCursor)
        self.searchButton.setObjectName("searchButton")

        self.startButton = QtWidgets.QPushButton(self.centralwidget)
        self.startButton.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes,
                self.button_width,
                self.box_height,
            )
        )
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.startButton.setFont(font)
        self.startButton.setCursor(Qt.CursorShape.PointingHandCursor)
        self.startButton.setObjectName("startButton")

        self.textKeywordRequest = QtWidgets.QLabel(self.centralwidget)
        self.textKeywordRequest.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.second_section_top + self.left_right_top_bottom_margin,
                700,
                40,
            )
        )
        font = QtGui.QFont()
        font.setPointSize(self.large_font_size)
        self.textKeywordRequest.setFont(font)
        self.textKeywordRequest.setObjectName("textKeywordRequest")

        self.textIdRequest = QtWidgets.QLabel(self.centralwidget)
        self.textIdRequest.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.third_section_top + self.left_right_top_bottom_margin,
                50,
                self.text_height,
            )
        )
        font = QtGui.QFont()
        font.setPointSize(self.large_font_size)
        self.textIdRequest.setFont(font)
        self.textIdRequest.setObjectName("textIdRequest")

        self.textNameRequest = QtWidgets.QLabel(self.centralwidget)
        self.textNameRequest.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin
                + int(
                    (
                        self.long_box_width
                        + self.distance_between_boxes
                        + self.button_width
                        - self.distance_between_boxes
                    )
                    / 2
                )
                + self.distance_between_boxes,
                self.third_section_top + self.left_right_top_bottom_margin,
                80,
                self.text_height,
            )
        )
        font = QtGui.QFont()
        font.setPointSize(self.large_font_size)
        self.textNameRequest.setFont(font)
        self.textNameRequest.setObjectName("textNameRequest")

        self.inputID = QtWidgets.QLineEdit(self.centralwidget)
        self.inputID.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.inputID.setFont(font)
        self.inputID.setObjectName("inputID")

        self.inputName = QtWidgets.QLineEdit(self.centralwidget)
        self.inputName.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.inputName.setFont(font)
        self.inputName.setObjectName("inputName")

        self.clearButton = QtWidgets.QPushButton(self.centralwidget)
        self.clearButton.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.clearButton.setFont(font)
        self.clearButton.setCursor(Qt.CursorShape.PointingHandCursor)
        self.clearButton.setObjectName("clearButton")

        self.goButton = QtWidgets.QPushButton(self.centralwidget)
        self.goButton.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.goButton.setFont(font)
        self.goButton.setCursor(Qt.CursorShape.PointingHandCursor)
        self.goButton.setObjectName("goButton")

        self.platzhalter = QtWidgets.QLabel(self.centralwidget)
        self.platzhalter.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin
                + self.long_box_width
                + self.distance_between_boxes,
                105,  # TODO
                self.button_width,
                int((1080 / 1920) * self.button_width),
            )
        )
        self.platzhalter.setText("")
        self.platzhalter.setObjectName("platzhalter")
        self.platzhalter.setScaledContents(True)

        self.platzhalter2 = QtWidgets.QLabel(self.centralwidget)
        self.platzhalter2.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin
                + self.long_box_width
                + self.distance_between_boxes,
                293,  # TODO
                self.button_width,
                int((1080 / 1920) * self.button_width),
            )
        )
        self.platzhalter2.setText("")
        self.platzhalter2.setObjectName("platzhalter2")
        self.platzhalter2.setScaledContents(True)

        self.textSentBy = QtWidgets.QLabel(self.centralwidget)
        self.textSentBy.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_box_and_text
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes
                + self.box_height
                + self.distance_to_next_paragraph
                + self.box_height
                + self.distance_between_boxes
                + self.box_height
                + self.distance_between_boxes,
                self.long_box_width,
                self.box_height,
            )
        )
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        font.setBold(True)
        font.setWeight(75)
        self.textSentBy.setFont(font)
        self.textSentBy.setObjectName("textSentBy")

        self.textRoutetThrough = QtWidgets.QLabel(self.centralwidget)
        self.textRoutetThrough.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_box_and_text
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes
                + self.box_height
                + self.distance_to_next_paragraph
                + self.box_height
                + self.distance_between_boxes,
                self.button_width,
                self.box_height,
            )
        )
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        font.setBold(True)
        font.setWeight(75)
        self.textRoutetThrough.setFont(font)
        self.textRoutetThrough.setObjectName("textRoutetThrough")

        self.boxSentBy = QtWidgets.QTextEdit(self.centralwidget)
        self.boxSentBy.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.large_font_size)
        self.boxSentBy.setFont(font)
        self.boxSentBy.setLineWrapMode(QtWidgets.QTextEdit.LineWrapMode.NoWrap)
        self.boxSentBy.setObjectName("boxSentBy")

        self.boxRoutetThrough = QtWidgets.QTextEdit(self.centralwidget)
        self.boxRoutetThrough.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.large_font_size)
        self.boxRoutetThrough.setFont(font)
        self.boxRoutetThrough.setLineWrapMode(QtWidgets.QTextEdit.LineWrapMode.NoWrap)
        self.boxRoutetThrough.setObjectName("boxRoutetThrough")

        self.startDialog = QtWidgets.QLabel(self.centralwidget)
        self.startDialog.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.left_right_top_bottom_margin,
                self.long_box_width,
                self.box_height,
            )
        )
        font = QtGui.QFont()
        font.setPointSize(self.large_font_size)
        self.startDialog.setFont(font)
        self.startDialog.setObjectName("startDialog")

        self.dropKM = QtWidgets.QComboBox(self.centralwidget)
        self.dropKM.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.dropKM.setFont(font)
        self.dropKM.setEditable(False)
        self.dropKM.setObjectName("dropKM")

        self.textKMRequest = QtWidgets.QLabel(self.centralwidget)
        self.textKMRequest.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_box_and_text,
                self.long_box_width,
                self.text_height,
            )
        )
        font = QtGui.QFont()
        font.setPointSize(self.large_font_size)
        self.textKMRequest.setFont(font)
        self.textKMRequest.setObjectName("textKMRequest")

        self.boxSourceBus = QtWidgets.QTextEdit(self.centralwidget)
        self.boxSourceBus.setGeometry(QtCore.QRect(0, 0, 0, 0))
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        self.boxSourceBus.setFont(font)
        self.boxSourceBus.setLineWrapMode(QtWidgets.QTextEdit.LineWrapMode.NoWrap)
        self.boxSourceBus.setObjectName("boxSourceBus")

        self.textboxSourceBus = QtWidgets.QLabel(self.centralwidget)
        self.textboxSourceBus.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_box_and_text
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes
                + self.box_height
                + self.distance_to_next_paragraph,
                self.button_width,
                self.box_height,
            )
        )
        font = QtGui.QFont()
        font.setPointSize(self.mid_font_size)
        font.setBold(True)
        font.setWeight(75)
        self.textboxSourceBus.setFont(font)
        self.textboxSourceBus.setObjectName("textboxSourceBus")

        PDU_Search.setCentralWidget(self.centralwidget)
        self.menubar = QtWidgets.QMenuBar(PDU_Search)
        self.menubar.setGeometry(QtCore.QRect(0, 0, 800, 26))
        self.menubar.setObjectName("menubar")

        PDU_Search.setMenuBar(self.menubar)
        self.statusbar = QtWidgets.QStatusBar(PDU_Search)
        self.statusbar.setObjectName("statusbar")

        PDU_Search.setStatusBar(self.statusbar)

        (
            self.list_df_kmatrix,
            self.list_file_path_split,
            self.list_all_bus,
            self.results,
        ) = (
            [],
            [],
            [],
            [],
        )
        # self.df_results_collected = pd.DataFrame()

        # Ladekreis starten
        gif_path = self.resource_path(f"{basedir}/../images/Loading.gif")
        pic_ready_path = self.resource_path(
            f"{basedir}/../images/porsche-model-gt3rs.png"
        )
        # pic_division_logo_path = self.resource_path(f"{basedir}/../images/Logo.png")
        self.gif = QMovie(gif_path)
        self.pic_ready = QPixmap(pic_ready_path)
        # self.pic_divisions_logo = QPixmap(pic_division_logo_path)

        self.retranslateUi(PDU_Search)
        QtCore.QMetaObject.connectSlotsByName(PDU_Search)

        # Buttons ihre Funktionen zuweisen
        self.searchButton.clicked.connect(self.search_clicked)
        self.startButton.clicked.connect(self.start_clicked)
        self.goButton.clicked.connect(self.go_clicked)
        self.clearButton.clicked.connect(self.clear_clicked)
        self.findButton.clicked.connect(self.find_clicked)
        # self.resetButton.clicked.connect(self.reset_clicked)

    def retranslateUi(self, PDU_Search):
        _translate = QtCore.QCoreApplication.translate
        PDU_Search.setWindowTitle(_translate("K-Matrix Toolkit", "K-Matrix Toolkit"))
        self.directory_path.setPlaceholderText(
            _translate("PDU_Search", "Bsp. C:\\User\\Files")
        )
        self.searchButton.setText(_translate("PDU_Search", "Search"))
        self.startButton.setText(_translate("PDU_Search", "Start"))
        self.startDialog.setText(_translate("PDU_Search", "Please select a directory"))
        # self.logoDivision.setPixmap(self.pic_divisions_logo)

    def search_clicked(self):
        """
        Open Dialoge to select Folder with Kmatrices
        """

        # print(self.resource_path(f"{basedir}/../images/Loading.gif"))
        self.directory = QtWidgets.QFileDialog.getExistingDirectory(
            None, "Select Folder"
        )
        self.directory_path.setText(self.directory)

    def start_clicked(self):
        """
        Program is started and the KMatrices are read out
        """
        # Clean up everything
        self.list_df_kmatrix = []
        self.new_list_df_kmatrix = []
        self.list_kmatrix_paths = []
        self.temp_list_file_path_split = []
        self.list_all_bus = []
        self.start_animation()
        self.dropKM.clear()
        self.inputID.setText("")
        self.inputName.setText("")
        self.boxSentBy.setText("")
        self.boxRoutetThrough.setText("")
        self.boxSourceBus.setText("")
        self.inputKeyword.setText("")

        directory_path = self.directory_path.text()

        # Save Data from KMatrices
        self.list_kmatrix_paths = functions.get_file_path(directory_path)

        worker = Worker(
            self.list_kmatrix_paths, None, None, None, None, True, os.cpu_count() - 1
        )
        self.threadpool.start(worker)
        worker.signals.results.connect(self.save_results)

    def save_results(self, results: list):
        """
        As soon as tables are read the rest of the user interface appears
        :param results: List where results of get:files are stored in
        """
        self.stop_animation()

        for result in results:
            self.list_df_kmatrix.append(result[0])
            self.list_file_path_split.append(result[1])
            self.list_all_bus.append(result[2])

        # Fill the combo box with the names of the KMatrices
        for list_file_path in self.list_file_path_split:
            for item in list_file_path:
                if ".xlsx" in item:
                    self.dropKM.addItem(item)

        # Copies contents of the KMatrices into a new list
        self.new_list_df_kmatrix = [0] * len(self.list_df_kmatrix)
        for idx, df_kmatrix in enumerate(self.list_df_kmatrix):
            self.new_list_df_kmatrix[idx] = df_kmatrix.copy()
        functions.row_to_header(self.new_list_df_kmatrix)

        self.inputID.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box,
                int(
                    (
                        self.long_box_width
                        + self.distance_between_boxes
                        + self.button_width
                        - self.distance_between_boxes
                    )
                    / 2
                ),
                self.box_height,
            )
        )
        self.inputName.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin
                + int(
                    (
                        self.long_box_width
                        + self.distance_between_boxes
                        + self.button_width
                        - self.distance_between_boxes
                    )
                    / 2
                )
                + self.distance_between_boxes,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box,
                int(
                    (
                        self.long_box_width
                        + self.distance_between_boxes
                        + self.button_width
                        - self.distance_between_boxes
                    )
                    / 2
                ),
                self.box_height,
            )
        )
        self.clearButton.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin
                + self.button_width
                + self.distance_between_boxes,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_box_and_text
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes,
                self.button_width,
                self.box_height,
            )
        )
        self.goButton.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_box_and_text
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes,
                self.button_width,
                self.box_height,
            )
        )
        self.boxSentBy.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin
                + self.button_width
                + self.distance_between_boxes,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_box_and_text
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes
                + self.box_height
                + self.distance_to_next_paragraph
                + self.box_height
                + self.distance_between_boxes
                + self.box_height
                + self.distance_between_boxes,
                self.long_box_width,
                self.box_height,
            )
        )
        self.boxRoutetThrough.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin
                + self.button_width
                + self.distance_between_boxes,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_box_and_text
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes
                + self.box_height
                + self.distance_to_next_paragraph
                + self.box_height
                + self.distance_between_boxes,
                self.long_box_width,
                self.box_height,
            )
        )
        # Drop down menu of found k-matrices
        self.dropKM.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_box_and_text
                + self.text_height
                + self.distance_between_text_and_box,
                self.long_box_width + self.distance_between_boxes + self.button_width,
                self.box_height,
            )
        )
        self.boxSourceBus.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin
                + self.button_width
                + self.distance_between_boxes,
                self.third_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_box_and_text
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes
                + self.box_height
                + self.distance_to_next_paragraph,
                self.long_box_width,
                self.box_height,
            )
        )
        self.checkBoxFreeTextSearch.setGeometry(
            QtCore.QRect(
                180,
                self.second_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes,
                self.button_width + 20,
                17,
            )
        )
        self.checkBoxCase.setGeometry(
            QtCore.QRect(
                335,
                self.second_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes,
                self.button_width + 20,
                17,
            )
        )
        self.inputKeyword.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.second_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box,
                self.long_box_width + self.distance_between_boxes + self.button_width,
                self.box_height,
            )
        )
        self.findButton.setGeometry(
            QtCore.QRect(
                self.left_right_top_bottom_margin,
                self.second_section_top
                + self.left_right_top_bottom_margin
                + self.text_height
                + self.distance_between_text_and_box
                + self.box_height
                + self.distance_between_boxes,
                self.button_width,
                self.box_height,
            )
        )

        self.textKeywordRequest.setText(
            "The search result is saved as an .xlsx file in the folder containing the K-matrices:"
        )
        self.textIdRequest.setText("ID:")
        self.textNameRequest.setText("Name: ")
        self.clearButton.setText("Clear")
        self.inputID.setPlaceholderText("Example: 0x11F")
        self.inputName.setPlaceholderText("Example: Aero_03")
        self.goButton.setText("Go!")
        self.textSentBy.setText("Sent by:")
        self.textRoutetThrough.setText("Routet through:")
        self.textKMRequest.setText("Select a K-Matrix:")
        self.textboxSourceBus.setText("Source Bus:")
        self.checkBoxFreeTextSearch.setText("Free text search")
        self.checkBoxCase.setText("Case insensitive")
        self.inputKeyword.setPlaceholderText("Example: Airbag")
        self.findButton.setText("Find")
        # self.resetButton.setText("Reset")

    def go_clicked(self):
        """
        Start search for source ECU, routing ECU and source Bus
        """
        signal_id = self.inputID.text()
        signal_id = signal_id.replace(" ", "")
        signal_id = signal_id.replace("\n", "")

        signal_name = self.inputName.text()
        signal_name = signal_name.replace(" ", "")
        signal_name = signal_name.replace("\n", "")

        selected_kmatrix = self.dropKM.currentText()

        temp_list_file_path_split = self.list_file_path_split.copy()

        id_kmatrix, list_file_path_split = functions.get_number_of_KMatrix(
            temp_list_file_path_split, selected_kmatrix
        )

        try:
            # Get name or ID of signal wheter one of them is not already given
            if self.inputID.text():
                signal_name = functions.get_signal_name(
                    signal_id,
                    self.new_list_df_kmatrix,
                    self.list_kmatrix_paths,
                    id_kmatrix,
                )
                self.inputName.setText(signal_name)
            else:
                signal_id = functions.get_signal_id(
                    signal_name,
                    self.new_list_df_kmatrix,
                    self.list_kmatrix_paths,
                    id_kmatrix,
                )
                self.inputID.setText(str(signal_id))

            # get information from KMatrix and fill it to the GUI
            (
                routing_ecus,
                source_ecus,
                source_buses,
            ) = functions.find_ecus_and_source_bus(
                self.new_list_df_kmatrix,
                list_file_path_split,
                signal_name,
                self.list_kmatrix_paths,
                self.list_all_bus,
            )
            self.boxRoutetThrough.setText(routing_ecus)
            self.boxSentBy.setText(source_ecus)
            self.boxSourceBus.setText(source_buses)

        # Popup warning if Signal isn't in selected PDU
        except IndexError as e:
            print(e)
            msg = QMessageBox()
            msg.setWindowTitle("Warning!")
            msg.setText(
                "Your search parameters did not appear in the selected K-Matrix!"
            )
            msg.setIcon(QMessageBox.Icon.Warning)

            msg.exec()

    def clear_clicked(self):
        """
        Method to clear the input fields
        """
        self.inputID.setText("")
        self.inputName.setText("")
        self.boxSentBy.setText("")
        self.boxRoutetThrough.setText("")
        self.boxSourceBus.setText("")

    def find_clicked(self):
        """
        Start search for keyword in every KMatrix
        """
        self.start_animation2()

        # Check what kind of search the user wants
        if self.checkBoxFreeTextSearch.isChecked():
            self.is_free_text_search = True
        else:
            self.is_free_text_search = False

        if self.checkBoxCase.isChecked():
            self.is_case_sensitive = True
        else:
            self.is_case_sensitive = False

        self.keyword = self.inputKeyword.text()

        self.temp_list_kmatrix_paths = self.list_kmatrix_paths.copy()

        worker = Worker(
            self.temp_list_kmatrix_paths,
            self.list_df_kmatrix,
            self.keyword,
            self.is_free_text_search,
            self.is_case_sensitive,
            False,
            os.cpu_count() - 1,
        )
        self.threadpool.start(worker)

        # Show results in Excel
        worker.signals.search_results.connect(self.show_excel)

        # Display warning if keyword wasn't found
        worker.signals.was_keyword_found.connect(self.show_warning)

    def show_excel(
        self,
        search_results: list,
    ):
        """
        Method to show table with found data
        :param df_final_kmatrix_result: Pandas Datframe containing all the data matching the keyword
        """

        df_final_kmatrix_result = search_results[0]
        df_user_inputs = search_results[1]
        df_kmatrix_names = search_results[2]

        # generate filename and save excel file
        timestr = time.strftime("%Y%m%d_%H%M%S")
        filename = f"{timestr}_Search_Results.xlsx"
        filepath = str(self.directory_path.text()) + "/" + filename

        print("Saving search results to Excel file...")
        with pd.ExcelWriter(filepath) as writer:
            df_final_kmatrix_result.to_excel(
                writer, sheet_name="Search_Results", index=False
            )
            df_user_inputs.to_excel(writer, sheet_name="User_Inputs", index=False)
            df_kmatrix_names.to_excel(
                writer, sheet_name="searched_KMatrix_Names", index=False
            )

        # Example usage:
        open_excel_file_in_excel(filepath)

        # Stop loading icon
        self.stop_animation2()

    # def reset_clicked(self):
    #     """
    #     Method to reset df_results_collected
    #     """
    #     self.df_results_collected = pd.DataFrame()

    def show_warning(self, was_keyword_found: bool):
        """
        Method to build a pop-up warning when keyword wasn't found
        :param was_keyword_found: Boolean wheter word was found or not
        """
        self.stop_animation2()
        if was_keyword_found:
            msg = QMessageBox()
            msg.setWindowTitle("Warning!")
            msg.setText("Suchwort nicht gefunden!")
            msg.setIcon(QMessageBox.Icon.Warning)
            msg.exec()

    # Methode um GIF bei der Konvertierung zu .exe zu behalten
    def resource_path(self, relative_path):
        """
        Function to keep GIF when converting to .exe
        """
        try:
            base_path = sys._MEIPASS
        except Exception:
            base_path = os.path.abspath(".")

        return os.path.join(base_path, relative_path)

    def start_animation(self):
        """
        Method to start loading duck while collecting data from the Excel Files
        """
        self.platzhalter.setMovie(self.gif)
        self.gif.start()

    def start_animation2(self):
        """
        Method to start loading duck while searching for keyword
        """
        self.platzhalter2.setMovie(self.gif)
        self.gif.start()

    def stop_animation(self):
        """
        Method to stop loading duck when finished collecting data from the Excel Files
        """
        self.gif.stop()
        self.platzhalter.setPixmap(self.pic_ready)

    def stop_animation2(self):
        """
        Method to stop loading duck when finished searching for keyword
        """
        self.gif.stop()
        self.platzhalter2.setPixmap(self.pic_ready)
