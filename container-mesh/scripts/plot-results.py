#!/usr/bin/env python3

import argparse
import math
import os.path
import pathlib
import re
from typing import NamedTuple, Dict, Tuple, List, Optional

import matplotlib.pyplot as plt

GRAPH_DPI = 300
GRAPH_SIZE = (12, 10)
MAKE_SCATTER_GIF = False

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

TestOutput = Dict[str, Tuple[TestInfo, Dict]]

# Returns a map of test_dir to (test_info, data_dict), where data_dict has path
# graph_type -> peer -> [data], and each data dict has keys matched by data_re below
def parse_test_output(results_dir: str) -> TestOutput:
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
            print("# --> file: %s" % f)
            m = path_re.match(f)
            assert m
            (graph_type, _) = m.groups()
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

def get_max_yval(stat_name: str, data: TestOutput):
    global MAKE_SCATTER_GIF
    max_y = 0
    if MAKE_SCATTER_GIF:
        # graph_type -> peer
        for test_dir, (info, data) in data.items():
            for graph_type, peer_map in data.items():
                for peer_id, iterations in peer_map.items():
                    for i in iterations:
                        if int(i[stat_name]) > max_y:
                            max_y = int(i[stat_name])
    return max_y

def usec_to_sec(usec: int) -> float:
    return float(usec) / 1.0e6

# returns filename of the graph crated
def scatter_by_peer(data: Dict[str, Dict[str, List[dict]]], output_dir: str,
                    info: TestInfo, max_y=None, log_y=False) -> str:
    global MAKE_SCATTER_GIF
    # plot a scatter chart with x axis as peer number, y axis as average
    # latency, and color corresponding to graph type

    stat = 'avg_usec'
    scatter_min_val = 1.1 if log_y else 0

    fig, _ax = plt.subplots(figsize=GRAPH_SIZE, dpi=GRAPH_DPI)
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
                yval = int(iteration[stat])
                yval = max(scatter_min_val, yval) if MAKE_SCATTER_GIF else yval
                y.append(usec_to_sec(yval))
        #type: ignore
        # zero values indicate failures, enlarge them
        y_to_size = lambda y: 20 if y < scatter_min_val else 5
        dot_sizes = [y_to_size(val)*4 if MAKE_SCATTER_GIF else y_to_size(val) for val in y]
        ax.scatter(x, y, s=dot_sizes, label=graph_type, color=graph_type_color(graph_type))
    ax.set_xlabel('Peer Number')
    ax.set_ylabel('Average Latency (sec)')
    if MAKE_SCATTER_GIF:
        ax.set_ylim(0, max_y)
        mag = math.ceil(math.log(num_peers, 10))
        plt.xticks(range(0, num_peers, 10*mag//2))
        plt.ylim(bottom=1.0 if log_y else 0.0)
        plt.yscale('log' if log_y else 'linear')

    if not MAKE_SCATTER_GIF:
        ax.legend()
    # save to png
    plt.title(f"Avg. latency w/ {num_peers} peers, {info.iterations} iter. of {info.duration_sec} sec.")
    fname = os.path.join(output_dir, f"sc-avg-latency-{num_peers:03d}p-{info.iterations}i-{info.duration_sec}s.png")
    fig.savefig(fname) #type: ignore
    return fname

def events_by_scale(tests: Dict[str, Tuple[TestInfo, Dict]], output_dir: str):
    average_by_scale(tests, 'num_events', 'Records read / second / peer',
                     True, output_dir)

def min_latency_by_scale(tests: Dict[str, Tuple[TestInfo, Dict]], output_dir: str):
    average_by_scale(tests, 'min_usec', "Min latency", False, output_dir, log_y=True)

def max_latency_by_scale(tests: Dict[str, Tuple[TestInfo, Dict]], output_dir: str):
    average_by_scale(tests, 'max_usec', "Max latency", False, output_dir)

def avg_latency_by_scale(tests: Dict[str, Tuple[TestInfo, Dict]], output_dir: str):
    average_by_scale(tests, 'avg_usec', "Average latency", False, output_dir)

def average_by_scale(tests: Dict[str, Tuple[TestInfo, Dict]], field_name,
                     y_description: str, per_second: bool, output_dir: str, log_y=False):
    # plot line graph of messages processed per second vs scale
    fig, _ax = plt.subplots(figsize=GRAPH_SIZE, dpi=GRAPH_DPI)
    ax: plt.Axes = _ax  # type: ignore

    # set of x and y values for each graph type
    # y value is average statistic per second over all peers, over all iterations
    x_scale: Dict[str, List[int]] = {}
    y_stat: Dict[str, List[float]] = {}
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
                y_stat[graph_type] = []
            x_scale[graph_type].append(info.scale)
            total_stat = 0
            for peer_id, iterations in peer_map.items():
                for iteration in iterations:
                    total_stat += int(iteration[field_name])
                if info.iterations != len(iterations):
                    warn_once(f"{test_dir} peer {peer_id} has {len(iterations)} iterations, expected {info.iterations}")
            total_stat /= info.iterations
            total_stat /= float(len(peer_map.keys())) # average over peers
            if per_second:
                total_stat /= float(info.duration_sec)
            y_stat[graph_type].append(total_stat)

    for graph_type in x_scale:
        xs = x_scale[graph_type]
        ys = y_stat[graph_type]
        # sort points by x value
        xs, ys = zip(*sorted(zip(xs, ys)))
        #type: ignore
        ax.plot(xs, ys, label=graph_type, color=graph_type_color(graph_type))
        ax.set_xlabel('Scale')
        ax.set_ylabel(y_description + (" (log)" if log_y else ""))
        ax.legend()
        plt.ylim(bottom=1.0 if log_y else 0.0)
        plt.yscale('log' if log_y else 'linear')
        plt.title(f"{field_name} vs Num peers, mean over {count} iter. of {duration} sec.")
        fname = os.path.join(output_dir, f"{field_name}-scale-{count}i-{duration}s.png")
        fig.savefig(fname) #type: ignore

def process_dot_files(results_dir: str) -> None:
    import pydot
    dot_files = pathlib.Path(results_dir).rglob('*.dot')
    for df in dot_files:
        # plot .svg of connection graph from .dot file
        out_dir = os.path.dirname(df)
        out_file = os.path.basename(df)
        try:
            (graph,) = pydot.graph_from_dot_file(df)
        except Exception as e:
            print("Failed to process dot file. Is graphviz installed?")
            raise e
        svg_filename = os.path.join(out_dir, str(out_file)[:-4] + '.svg')
        graph.write_svg(svg_filename)
        print(f"--> wrote {svg_filename}")

def main():
    global MAKE_SCATTER_GIF
    info = "Reads all data-*.log files in <results_dir> and outputs graphs to <output_dir>"
    parser = argparse.ArgumentParser(description="cmesh results graph plotter",
                                    epilog=info)
    parser.add_argument("-o", "--output-dir", help="output directory", default=".")
    parser.add_argument("-g", "--gif", action="store_true", help="make scatter gif")
    parser.add_argument("-d", "--process-dot", action="store_true", help="Process any .dot files found")
    parser.add_argument('results_dir', type=str, help="directory containing data-*.log files")
    args=parser.parse_args()
    if args.gif:
        MAKE_SCATTER_GIF = True

    tests = parse_test_output(args.results_dir)
    max_y : Optional[int] = None
    log_y = False
    if MAKE_SCATTER_GIF:
        max_y = math.ceil(usec_to_sec(get_max_yval('avg_usec', tests)))
        print(f"# --> using fixed y range (max {max_y})")
        #log_y = True

    scatter_filenames = []
    for _, (info, data) in tests.items():
        scatter_filenames.append(scatter_by_peer(data, args.output_dir, info, max_y=max_y, log_y=log_y))
    scatter_filenames.sort()

    # make a gif
    if MAKE_SCATTER_GIF:
        import imageio
        images = []
        base_dir = None
        for filename in scatter_filenames:
            images.append(imageio.imread(filename))
            if not base_dir:
                base_dir = os.path.dirname(filename)
        imageio.mimsave(os.path.join(base_dir or "./", 'peers-avg-lat.gif'), images, loop=0, fps=0.9)


    # other graph ideas:
    # messages processed per second per vs scale
    events_by_scale(tests, args.output_dir)
    min_latency_by_scale(tests, args.output_dir)
    max_latency_by_scale(tests, args.output_dir)
    avg_latency_by_scale(tests, args.output_dir)

    if args.process_dot:
        process_dot_files(args.results_dir)


if __name__ == "__main__":
    main()
