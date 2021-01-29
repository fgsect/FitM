#!/usr/bin/python3
import sys
import os
import re

replayable_queue = sys.argv[1]
region_folder = sys.argv[2]
output_folder_name = "parsed_regions"

if not os.path.isdir(output_folder_name):
    os.mkdir(output_folder_name)
else:
    print("out folder already exists")

for filename in os.listdir(replayable_queue):
    input_file = open(os.path.join(replayable_queue, filename), 'rb')
    regions_file = open(os.path.join(region_folder, filename), 'r')

    regions = regions_file.read()

    # create dir input.file_name
    parsed_ouput_path = os.path.join(output_folder_name, filename)
    os.mkdir(parsed_ouput_path)
    regions = regions.split('\n')
    for region in regions:
        matches = re.match(r'Region (\d+) - Start: (\d+), End: (\d+)', region)
        if not matches:
            continue

        id = matches.group(1)
        start = int(matches.group(2))
        end = int(matches.group(3))
        # create file for id
        id_file = open(os.path.join(parsed_ouput_path, id), 'wb')

        output = input_file.read(end-start)

        id_file.write(output)

    regions_file.close()
    input_file.close()
