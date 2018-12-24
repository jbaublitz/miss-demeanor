#!/usr/bin/python

import json
import sys

method = sys.argv[1]
uri = sys.argv[2]
headers = json.loads(sys.argv[3])
body = sys.argv[4]

print(method)
print(uri)
print(headers)
print(body)
