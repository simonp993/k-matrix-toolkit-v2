import qdarktheme
from PyQt6 import QtWidgets

from src.K_Matrix_Tool_Layout import Ui_PDU_Search

if __name__ == "__main__":
    import sys
    from multiprocessing import freeze_support

    freeze_support()

    app = QtWidgets.QApplication(sys.argv)
    qdarktheme.setup_theme("auto")

    PDU_Search = QtWidgets.QMainWindow()
    PDU_Search.setAutoFillBackground(True)
    ui = Ui_PDU_Search()
    ui.setupUi(PDU_Search)
    PDU_Search.show()
    sys.exit(app.exec())
