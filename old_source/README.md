# K-Matrix-Toolkit
<small><small>(Melvin Kothe, melvin.kothe2@porsche.de, 08.02.2023)</small></small>
<small><small>(Simon Pavicic, simon.pavicic@porsche.de, 07.05.2024)</small></small>

<hr>

## Getting started

This repository contains all necessary information and scripts to install the K-Matrix Toolkit.

Confluence documentation: [Confluence K-Matrix-Toolkit](https://skyway.porsche.com/confluence/display/EIC2/K-Matrix+Toolkit)
<hr> 

## Installation Apple Silicon

To install the application on your apple silicon Mac follow these steps:

1. Clone this repository:
```
#ssh
git clone git@cicd.skyway.porsche.com:PMAY/k-matrix-toolkit.git

#https
git clone https://cicd.skyway.porsche.com/PMAY/k-matrix-toolkit.git
```

2. Right click on the cloned repository directory, press and hold <kbd>option⌥</kbd> and select <kbd>Copy "K-Matrix-Toolkit" as Pathname</kbd>
3. Open a Terminal window and navigate to your cloned directory by entering <kbd>cd</kbd> then <kbd>spacebar</kbd> then <kbd>command⌘+V</kbd>
```
cd /Users/<your_user_name>/Desktop/Agent-K Anpassung/K-Matrix-Toolkit
```
4. Execute the following command in your terminal:
```
sh scripts/build_dmg_silicon_mac.sh
```
5. A <kbd>K-Matrix-Tool.dmg</kbd> file and a <kbd>K-Matrix-Tool.app</kbd> file appear in the cloned directory. 
6. You can drag and drop the <kbd>K-Matrix-Tool.app</kbd> file to your applications folder and use it as a normal app on your Mac.
7. Use the <kbd>K-Matrix-Tool.dmg</kbd> file to share the app to other apple silicon users.

### Development MacOS
To quickly develop and see any changes made to the source code, please first install Python 3.9.6 (inside the virtual env)
Then navigate to the main directory ```k-matrix-toolkit``` in your terminal and execute the following code:

```
python3 -m ensurepip
python3 -m pip install virtualenv
python3 -m venv ./venv_dev    
source ./venv_dev/bin/activate   

pip install -U pip
pip install -U setuptools
pip install -r requirements.txt

python K_Matrix_Tool_APP.py
```

This will create a venv virtual env named ```venv_dev```and install the requirements inside it. 
Afterward the ```main()``` function is called. 

<hr>


## Installation Windows
To build the .exe file for Windows computers please do the following:
1. Install Python 3.9.6 using this link: https://www.python.org/downloads/release/python-396/
2. Install Git using this link: https://git-scm.com/download/win 

After installing Python go to the ```k-matrix-toolkit``` directory in the File Explorer and perform a right click. 
Choose ```Open Git Bash here``` and execute the .sh script by simply typing ```scripts\\build_exe_windows.sh```. 
This will build the .exe file for you. 


### Development Windows
To quickly develop and see any changes made to the source code, please first follow steps 1. and 2. from above and then execute the folloing inside the Git Bash Terminal:

```
python -m ensurepip
python -m pip install virtualenv
python -m venv .\\scripts\\venv_windows  
source .\\scripts\\venv_windows\\Scripts\\activate   

pip install -U pip
pip install -U setuptools
pip install -U pyinstaller
pip install -r requirements.txt

python K_Matrix_tool_APP.py
```