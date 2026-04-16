#!/bin/sh

python -m ensurepip
python -m pip install virtualenv
python -m venv .\\scripts\\venv_windows  
source .\\scripts\\venv_windows\\Scripts\\activate   

pip install -U pip
pip install -U setuptools
pip install -U pyinstaller
pip install -r requirements.txt

pyinstaller --onefile --name="K-Matrix-Tool" --add-data "images;images" --add-data "src;src" --windowed --icon=images\\porsche-model-gt3rs-logo.ico K_Matrix_Tool_APP.py
cp -r dist\\K-Matrix-Tool.exe .\\