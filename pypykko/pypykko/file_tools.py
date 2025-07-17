import json
import pathlib
import os
scripts_path = pathlib.Path(__file__).parent.resolve()


def get_filepath(filename, directory):
	if not directory:
		return filename
	filepath = pathlib.Path(os.path.join(scripts_path, directory, filename)).resolve()
	return filepath


def read_tsv(filename, directory=''):

	filename = get_filepath(filename, directory)

	table = []
	with open(filename, 'r') as file:
		for line in file:
			line = line.strip('\n')
			if not line:
				continue
			if line.startswith('#'):
				continue
			row = ['' if val == '-' else val for val in line.split('\t')]
			table.append(row)
	return table


def read_list_tsv(filename):
	return read_tsv(filename, directory='lists')


def save_tsv(filename, data, directory=''):
	filename = get_filepath(filename, directory)
	text = '\n'.join('\t'.join(val or '-' for val in row) for row in data) + '\n'
	save_txt(filename, text=text)


def read_txt(filename, directory=''):
	filename = get_filepath(filename, directory)
	with open(filename, 'r') as file:
		return file.read()


def save_txt(filename, text: str, directory=''):
	filename = get_filepath(filename, directory)
	with open(filename, 'w') as file:
		file.write(text)


def read_list(filename, directory=''):
	filename = get_filepath(filename, directory)
	return [s for s in read_txt(filename).splitlines() if s]


def save_list(filename, items: list, sort=True, directory=''):
	filename = get_filepath(filename, directory)
	items = sorted(items) if sort else items
	return save_txt(filename, '\n'.join(items))


def load_json(filename, directory=''):
	filename = get_filepath(filename, directory)
	with open(filename, 'r') as file:
		data = json.load(file)
	return data
