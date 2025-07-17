#! /usr/bin/env python3

"""
Tokenize text.
For testing/debugging purposes only.
"""

import re
import sys
from .constants import SENT_BREAK, LINE_BREAK
from .file_tools import read_list_tsv

UNITS = {re.escape(row[1]) for row in read_list_tsv('aux-units.tsv')}
EMOTICONS = {re.escape(row[1]) for row in read_list_tsv('aux-emoticons.tsv')}
ABBREV = {re.escape(row[1]) for row in read_list_tsv('aux-abbreviations.tsv')}
ABBREV = ABBREV | {a.capitalize() for a in ABBREV}

REGEX_ABBREV = '|'.join(ABBREV)
REGEX_EMOTICON = '|'.join(EMOTICONS)
REGEX_UNITS = '|'.join(UNITS)
REGEX_INITIAL = r'(?:[A-ZÅÄÖ][.])+'
REGEX_ORDINAL = r'[1-9][0-9]?[.]'
REGEX_DATE = r'(?:[1-9]|[12][0-9]|30|31).(?:[1-9]|10|11|12).(?:[0-9][0-9][0-9][0-9])?'
REGEX_CLOCK = r'(?:[0-9]?[0-9][:.][0-9][0-9])'
REGEX_HASHTAG = r'#[A-Za-z0-9_]+'
REGEX_HANDLE = r'@[A-Za-z0-9_]+'
REGEX_REDDIT = r'r/[A-Za-z0-9_]+|u/[A-Za-z0-9_]+'
REGEX_THOUSANDS = r'[1-9][0-9]?[0-9]?(?: [0-9][0-9][0-9])+(?:-[a-zåäö-]+)?'
REGEX_THOUSANDS_RANGE = r'[1-9][0-9]?[0-9]?(?: [0-9][0-9][0-9])+[-–][1-9][0-9]?[0-9]?(?: [0-9][0-9][0-9])+'
REGEX_XML_ELEM = r'<[^<>]+>'
REGEX_HTML_ENTITY = r'&[^;\s]+;'
# REGEX_URL = '(?:https?://|file:///)[a-z0-9](?:[.][a-z0-9][a-z0-9]+)+'
# REGEX_EMAIL = '(?:https?://|file:///)[a-z0-9](?:[.][a-z0-9][a-z0-9]+)+'
# REGEX_CHORD = 'xxx'
# REGEX_IUPAC_NAME = 'xxx'

REGEX_ALL = f'{LINE_BREAK}' \
			f'{REGEX_UNITS}|' \
			f'{REGEX_EMOTICON}|' \
			f'{REGEX_ABBREV}|' \
			f'{REGEX_INITIAL}|' \
			f'{REGEX_ORDINAL}|' \
			f'{REGEX_DATE}|' \
			f'{REGEX_CLOCK}|' \
			f'{REGEX_HASHTAG}|' \
			f'{REGEX_HANDLE}|' \
			f'{REGEX_REDDIT}|' \
			f'{REGEX_THOUSANDS}|' \
			f'{REGEX_THOUSANDS_RANGE}|' \
			f'{REGEX_XML_ELEM}|' \
			f'{REGEX_HTML_ENTITY}'

PUNCT_HEAD = '\t\n (/"“”„¿¡‹«»{[\'’'
PUNCT_TAIL = '\t\n .…,;?!)/"“”„›»}\\]\'’'


def text2tokens(text):

	def separate_punct(s):

		if not s:
			return []

		head = []
		tail = []
		while s and s[0] in PUNCT_HEAD:
			head.append(s[0])
			s = s[1:]
		while s and s[-1] in PUNCT_TAIL + ':':
			tail.append(s[-1])
			s = s[:-1]
		head.append(s) if s else None
		# TODO: Single quotes?
		separated = head + tail[::-1]
		return separated

	text = f' {text} '
	text = re.sub(rf'({REGEX_XML_ELEM})', r' \1 ', text)
	text = text.replace('\n\n', f' {LINE_BREAK} ')
	text = re.sub(r'(--+|==+|\.\.\.+)', r' \1 ', text)
	text = re.sub(r'\s+', ' ', text)
	segments = re.split(rf'(?<=[{PUNCT_HEAD}])({REGEX_ALL})(?=[{PUNCT_TAIL}]|:[^a-zåäö])', text)

	tokens = []
	for i, seg in enumerate(segments):
		if i % 2:
			tokens += [seg]
			continue
		for w in re.split(r'(\s+|/)', seg):
			tokens += separate_punct(w)

	return tokens


def text2sentences(text):

	def add_sentbreak():
		sentences.append([])

	sentences = [[]]
	tokens = text2tokens(text)

	for i in range(1, len(tokens) - 1):

		curr_token = tokens[i]
		prev_token = tokens[i - 1]
		next_token = tokens[i + 1]
		prev_context = ''.join(tokens[:i])
		next_context = ''.join(tokens[i+1:])

		if curr_token == LINE_BREAK:
			sentences.append([])
			continue

		if curr_token in ['–', '-'] and re.fullmatch(r'.+\s', prev_context) and re.fullmatch(r'\s[A-ZÅÄÖ].+', next_context):
			add_sentbreak()

		sentences[-1].append(curr_token)

		if curr_token == ' ' and prev_token in ['?', '!', '.', '…']:
			add_sentbreak()

		if next_token == ' ' and prev_token in ['?', '!', '.', '…'] and curr_token in ['"', '”', '»']:
			add_sentbreak()

	sentences = [
		[token for token in sent if token != ' '] for sent in sentences if sent
	]

	return sentences


def tokenize(text):
	output = ''
	sentences = text2sentences(text)
	for sent in sentences:
		if not sent:
			continue
		for token in sent:
			output += token + '\n'
		output += SENT_BREAK
		output += '\n'
	return output


if __name__ == '__main__':
	for line in sys.stdin:
		line = line.replace('&amp; ', '&').replace('&lt; ', '<').replace('&gt; ', '>')
		print(tokenize(line), end="")





