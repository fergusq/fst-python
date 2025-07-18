#! /usr/bin/env python3

"""
Parse token-per-line input.
"""

import sys
from argparse import ArgumentParser
from .utils import analyze
from .constants import PARSER_FST_PATH
import kfst

parser_fst = kfst.FST.from_kfst_file(PARSER_FST_PATH)

argparser = ArgumentParser(description='Morphologically analyze token-per-line input.')
argparser.add_argument('-o', '--only_best', action='store_true')
args = argparser.parse_args()

if __name__ == '__main__':
	for line in sys.stdin:
		wordform = line.strip()
		analyses = analyze(wordform, only_best=args.only_best)
		for analysis in analyses:
			print('%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s' % analysis)
		print()
