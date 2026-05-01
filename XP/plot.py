import sys
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib as mpl

# --- LNCS-style Plot Config ---
mpl.rcParams.update({
    "text.usetex": False,         
    "font.family": "sans-serif",
    "font.sans-serif": ["Helvetica", "Arial"],
    "axes.labelsize": 15,
    "font.size": 15,
    "legend.fontsize": 13,
    "xtick.labelsize": 13,
    "ytick.labelsize": 13,
    "figure.dpi": 300,
    "lines.linewidth": 1.0,
    "lines.markersize": 4,
    "axes.titlesize": 15
})
def load_csv(filepath):
    return pd.read_csv(filepath, header=None)

def plot_graph_1(tc_data, jade_data, shadow_data, nb_pkts):
    plt.figure(figsize=(5.5, 4.0))
    plt.plot(tc_data[0], tc_data[1], color="tab:blue", marker='x', label=r"tc (pure emulation)")
    plt.plot(jade_data[0], jade_data[1], color="tab:orange", marker='.', label=r"JADE")
    plt.plot(shadow_data[0], shadow_data[1], color="tab:green", marker='1', label=r"Shadow")
    plt.xlabel("Emulated Link Latency (ms)")
    plt.ylabel("Execution Time (s)")
    plt.title("Execution Time vs Link Latency\n({} requests & responses)".format(nb_pkts))
    plt.grid(True)
    plt.legend()
    plt.tight_layout()
    plt.savefig("latency_vs_time_{}pkts.pdf".format(nb_pkts))

def plot_graph_2(tc_1ms, jade_multi, shadow_multi):
    jade_avg = pd.DataFrame()
    jade_avg["requests"] = jade_multi[0]
    jade_avg["mean"] = jade_multi.loc[:, 1:5].mean(axis=1)
    jade_avg["std"] = jade_multi.loc[:, 1:5].std(axis=1)
    shadow_avg = pd.DataFrame()
    shadow_avg["requests"] = shadow_multi[0]
    shadow_avg["mean"] = shadow_multi.loc[:, 1:5].mean(axis=1)
    shadow_avg["std"] = shadow_multi.loc[:, 1:5].std(axis=1)
    plt.figure(figsize=(5.5, 4.0))
    plt.plot(tc_1ms[0], tc_1ms[1], marker='x', label=r"tc (1ms)")
    plt.errorbar(
        jade_avg["requests"], jade_avg["mean"], yerr=jade_avg["std"],
        fmt='-D', capsize=3, marker=",", label=r"JADE (mean +- std dev)"
    )
    plt.errorbar(
        shadow_avg["requests"], shadow_avg["mean"], yerr=shadow_avg["std"],
        fmt='-D', capsize=3, marker="1", label=r"shadow (mean +- std dev)"
    )

    plt.xlabel("Number of Requests")
    plt.ylabel("Execution Time (s)")
    plt.title("Execution Time vs Number of Requests\n(tc: 1ms, JADE and Shadow: latency agnostic)")
    plt.grid(True)
    plt.legend()
    plt.tight_layout()
    plt.savefig("requests_vs_time.pdf")

def main():
    if len(sys.argv) != 3:
        print("Usage: python plot.py create_latency create_nb_packets")
        sys.exit(1)
    
    if sys.argv[1] == "1":
        tc_path = "tc_{pkt}.csv"
        jade_path = "jade_{pkt}.csv"
        shadow_path = "shadow_{pkt}.csv"
        for pkt in [500]:
            latency_tc = load_csv(tc_path.format(pkt = pkt))
            latency_jade = load_csv(jade_path.format(pkt = pkt))
            latency_shadow = load_csv(shadow_path.format(pkt = pkt))
            plot_graph_1(latency_tc, latency_jade, latency_shadow, pkt)

    if sys.argv[2] == "1":
        tc_1ms = load_csv("tc_nb1.csv")
        jade_data = load_csv("jade_nb_5.csv")
        shadow_data = load_csv("shadow_nb_5.csv")
        plot_graph_2(tc_1ms, jade_data, shadow_data)


if __name__ == "__main__":
    main()
