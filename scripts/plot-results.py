#!/usr/bin/env python3

import glob
import os.path
import re
import sys

import matplotlib.pyplot as plt

# Slurp up all data-*.log files in results_dir then use matplotlib to make
# pretty pictures.

# Example format of data-*log files:
# # peer9-report.json PeerReport { message_latency: LatencyStats { num_events: 684, min_usec: 182130, max_usec: 2624592, avg_usec: 1017258, distinct_peers: 19 }, records_produced: 40 }
def parse_test_output(results_dir: str) -> dict:

    # one file per test run data-<graph_type>-<iteration>.log
    files = glob.glob(os.path.join(results_dir, 'data-*.log'))
    path_re = re.compile(r'.*data-(?P<graph_type>[^\.]+)-(?P<iteration>\d+).log')
    peer_re = re.compile(r'# peer(?P<peer>\d+)-report.json')
    data_re = re.compile(r'.*message_latency: LatencyStats { num_events: (?P<num_events>\d+), ' + \
            r'min_usec: (?P<min_usec>\d+), max_usec: (?P<max_usec>\d+), avg_usec: (?P<avg_usec>\d+), ' + \
            r'distinct_peers: (?P<distinct_peers>\d+) }, records_produced: (?P<records_produced>\d+)')
    outd = {}
    for f in files:
        print("file: %s" % f)
        m = path_re.match(f)
        assert m
        (graph_type, iteration) = m.groups()
        outd[graph_type] = {}
        outd[graph_type][iteration] = {}
        with open(f, 'r') as f:
            peer = -1
            for line in f:
                m = peer_re.match(line)
                if m:
                    peer = m.group('peer')
                else:
                    m = data_re.match(line)
                    if not m:
                        print(f"ERROR: {line} did not match {data_re}")
                        continue
                    outd[graph_type][iteration][peer] = m.groupdict()

    return outd

def graph_type_color(type: str) -> str:
    if type == 'complete':
        return 'red'
    elif type == 'spanning-tree':
        return 'blue'
    elif type == 'la-model':
        return 'green'
    else:
        return 'black'

def main(output_dir: str):
    # TODO cli param
    data = parse_test_output(output_dir)
    # plot a scatter chart with x axis as peer number, y axis as average
    # latency, and color corresponding to graph type
    fig, ax = plt.subplots()
    num_peers = -1
    for graph_type in data:
        x = []
        y = []
        num_peers = 0
        for iteration in data[graph_type]:
            for peer in data[graph_type][iteration]:
                num_peers += 1
                x.append(int(peer))
                y.append(int(data[graph_type][iteration][peer]['avg_usec']))
        ax.scatter(x, y, label=graph_type, color=graph_type_color(graph_type))
    ax.set_xlabel('Peer Number')
    ax.set_ylabel('Average Latency (usec)')
    ax.legend()
    # save to png
    plt.title(f"Avg. latency w/ {num_peers} peers")
    fig.savefig(f"avg-latency-{num_peers}p.png")



if __name__ == "__main__":
    main(sys.argv[1])
