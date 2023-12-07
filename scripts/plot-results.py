#!/usr/bin/env python3

import argparse
import glob
import os.path
import re
from typing import NamedTuple, Dict, Tuple, List

import matplotlib.pyplot as plt

# Slurp up all data-*.log files in results_dir then use matplotlib to make
# pretty pictures.

class TestInfo(NamedTuple):
    duration_sec: int
    iterations: int
    scale: int

def get_test_info(results_dir: str) -> TestInfo:
    duration_sec = iterations = scale = -1
    duration_re = re.compile(r'.*--test-duration-sec (\d+)')
    iterations_re = re.compile(r'^scale (\d+), iterations (\d+)')
    with open(os.path.join(results_dir, 'test_info.log'), 'r') as f:
        for _, line in zip(range(10), f):
            m = duration_re.match(line)
            if m:
                duration_sec = int(m.group(1))
            else:
                m = iterations_re.match(line)
                if m:
                    iterations = int(m.group(2))
                    scale = int(m.group(1))

    return TestInfo(duration_sec, iterations, scale)

# Example format of data-*log files:
# # peer9-report.json
# PeerReport { message_latency: LatencyStats { num_events: 684, min_usec: 182130, max_usec: 2624592, avg_usec: 1017258, distinct_peers: 19 }, records_produced: 40 }

# Returns a map of test_dir to (test_info, data_dict), where data_dict has path
# graph_type -> peer -> [data], and each data dict has keys matched by data_re below
def parse_test_output(results_dir: str) -> Dict[str, Tuple[TestInfo, Dict]]:
    # one file per test run data-<graph_type>-<iteration>.log
    path_re = re.compile(r'.*data-(?P<graph_type>[^\.]+)-(?P<iteration>\d+).log')
    peer_re = re.compile(r'# peer(?P<peer>\d+)-report.json')
    data_re = re.compile(r'.*message_latency: LatencyStats { num_events: (?P<num_events>\d+), ' + \
            r'min_usec: (?P<min_usec>\d+), max_usec: (?P<max_usec>\d+), avg_usec: (?P<avg_usec>\d+), ' + \
            r'distinct_peers: (?P<distinct_peers>\d+) }, records_produced: (?P<records_produced>\d+)')

    # find each test run dir and the files within it
    tests: Dict[str, Tuple[TestInfo, List[str]]] = {}
    for (root, _, files) in os.walk(results_dir):
        data_files = [f for f in files if path_re.match(f)]
        if data_files:
            tests[root] = (get_test_info(root), [os.path.join(root, df) for df in data_files])

    outd = {}
    # each test dir runs a single scale value with files for each iteration * graph type
    for test_dir, (info, files) in tests.items():
        testd = {}
        for f in files:
            print("file: %s" % f)
            m = path_re.match(f)
            assert m
            (graph_type, iteration_num) = m.groups()
            if testd.get(graph_type) is None:
                testd[graph_type] = {}
            with open(f, 'r') as f:
                peer = -1
                for line in f:
                    m = peer_re.match(line)
                    if m:
                        peer = m.group('peer')
                        if testd[graph_type].get(peer) is None:
                            testd[graph_type][peer] = []
                    else:
                        m = data_re.match(line)
                        if not m:
                            print(f"ERROR: {line} did not match {data_re}")
                            continue
                        testd[graph_type][peer].append(m.groupdict())
        outd[test_dir] = (info, testd)
    return outd

warned = False
def warn_once(msg: str):
    global warned
    if not warned:
        print(f"WARN: {msg}")
        warned = True

def graph_type_color(type: str) -> str:
    if type == 'complete':
        return 'red'
    elif type == 'spanning-tree':
        return 'blue'
    elif type == 'la-model':
        return 'green'
    else:
        return 'black'

def scatter_by_peer(data: Dict[str, Dict[str, List[dict]]], output_dir: str, info: TestInfo):
    # plot a scatter chart with x axis as peer number, y axis as average
    # latency, and color corresponding to graph type
    fig, _ax = plt.subplots()
    ax: plt.Axes = _ax  # type: ignore
    num_peers = -1
    for graph_type, peers in data.items():
        x = []
        y = []
        num_peers = 0
        for (peer, iterations) in peers.items():
            num_peers += 1
            for iteration in iterations:
                x.append(int(peer))
                y.append(int(iteration['avg_usec']))
        #type: ignore
        ax.scatter(x, y, label=graph_type, color=graph_type_color(graph_type))
    ax.set_xlabel('Peer Number')
    ax.set_ylabel('Average Latency (usec)')
    ax.legend()
    # save to png
    plt.title(f"Avg. latency w/ {num_peers} peers, {info.iterations} iter. of {info.duration_sec} sec.")
    fname = os.path.join(output_dir, f"sc-avg-latency-{num_peers}p-{info.iterations}i-{info.duration_sec}s.png")
    fig.savefig(fname) #type: ignore

def events_by_scale(tests: Dict[str, Tuple[TestInfo, Dict]], output_dir: str):
    # plot line graph of messages processed per second vs scale
    fig, _ax = plt.subplots()
    ax: plt.Axes = _ax  # type: ignore

    # set of x and y values for each graph type
    # y value is average events per second over all peers, over all iterations
    x_scale: Dict[str, List[int]] = {}
    y_events: Dict[str, List[float]] = {}
    count = duration = 0
    for test_dir, (info, data) in tests.items():
        if count == 0:
            count = info.iterations
            duration = info.duration_sec
        elif count != info.iterations or duration != info.duration_sec:
            print("WARN: iterations / duration mismatch between test runs, graph title may be inaccurate.")
        for graph_type, peer_map in data.items():
            if x_scale.get(graph_type) is None:
                x_scale[graph_type] = []
                y_events[graph_type] = []
            x_scale[graph_type].append(info.scale)
            total_events = 0
            for peer_id, iterations in peer_map.items():
                for iteration in iterations:
                    total_events += int(iteration['num_events'])
                if info.iterations != len(iterations):
                    warn_once(f"{test_dir} peer {peer_id} has {len(iterations)} iterations, expected {info.iterations}")
                total_events /= info.iterations
            total_events /= len(peer_map.keys()) # average over peers
            y_events[graph_type].append(total_events / float(info.duration_sec))

    for graph_type in x_scale:
        xs = x_scale[graph_type]
        ys = y_events[graph_type]
        # sort points by x value
        xs, ys = zip(*sorted(zip(xs, ys)))
        #type: ignore
        ax.plot(xs, ys, label=graph_type, color=graph_type_color(graph_type))
        ax.set_xlabel('Scale')
        ax.set_ylabel('Records read / second / peer')
        ax.legend()
        plt.title(f"Events/sec vs Num peers, {count} iter. of {duration} sec.")
        fname = os.path.join(output_dir, f"events-scale-{count}i-{duration}s.png")
        fig.savefig(fname) #type: ignore

def main():
    info = "Reads all data-*.log files in <results_dir> and outputs graphs to <output_dir>"
    parser = argparse.ArgumentParser(description="cmesh results graph plotter",
                                    epilog=info)
    parser.add_argument("-o", "--output-dir", help="output directory", default=".")
    parser.add_argument('results_dir', type=str, help="directory containing data-*.log files")
    args=parser.parse_args()

    tests = parse_test_output(args.results_dir)
    for _, (info, data) in tests.items():
        scatter_by_peer(data, args.output_dir, info)

    # other graph ideas:
    # messages processed per second per vs scale
    events_by_scale(tests, args.output_dir)

    # min/max/avg latency vs scale
    #latency_by_scale(data, args.output_dir, info)


if __name__ == "__main__":
    main()
