set -e 1
# Cargo tests

cd ../kfst-rs; cargo test; cd ../general-tests

# These tests want both kfst_py and kfst_rs installed

pip install ../kfst-rs
pip install ../kfst

python api_test.py
python format_test.py

# Needs to be run with and without kfst-rs installed

python deptest.py
python import_test.py
python test_pypykko.py
python symbol_test.py
python test_alignment_pypykko.py
python test_pypykko_components.py

pip uninstall kfst-rs -y

python deptest.py
python import_test.py
python test_pypykko.py
python symbol_test.py
python test_alignment_pypykko.py
python test_pypykko_components.py
