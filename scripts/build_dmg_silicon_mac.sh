#!/bin/sh

cd create-dmg
make install
cd ..
python3 -m ensurepip
python3 -m pip install virtualenv
python3 -m venv ./venv
source ./venv/bin/activate
pip install -U pip
pip install -U setuptools
pip install -r requirements.txt
pyinstaller --name="K-Matrix Toolkit" --add-data "images:images" --add-data "src:src" --windowed --target-arch arm64 --icon=images/porsche-model-gt3rs-logo.icns --noconfirm K_Matrix_Tool_APP.py
mkdir -p dist/dmg
cp -r "dist/K-Matrix Toolkit.app" dist/dmg
test -f "dist/K-Matrix Toolkit.dmg" && rm "dist/K-Matrix Toolkit.dmg"
create-dmg \
  --sandbox-safe \
  --volname "K-Matrix Toolkit" \
  --volicon "images/porsche-model-gt3rs-logo.icns" \
  --window-pos 200 120 \
  --window-size 600 300 \
  --icon-size 100 \
  --icon "K-Matrix Toolkit.app" 175 120 \
  --hide-extension "K-Matrix Toolkit.app" \
  --app-drop-link 425 120 \
  "dist/K-Matrix Toolkit.dmg" \
  "dist/dmg/"

cp -r "dist/K-Matrix Toolkit.dmg" ./
cp -r "dist/dmg/K-Matrix Toolkit.app" ./
rm -f -r dist/
rm -f -r build/
rm -f "K-Matrix Toolkit.spec"
deactivate
rm -f -r venv
