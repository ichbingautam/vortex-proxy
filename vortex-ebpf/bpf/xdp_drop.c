#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/in.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_endian.h>

// BPF Map to hold the list of blocked IPs (keys are IPv4 addresses as u32)
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 100000);
    __type(key, __u32);
    __type(value, __u32); // e.g. 1 means block
} BLOCKED_IPS SEC(".maps");

SEC("xdp_drop_ips")
int xdp_prog(struct xdp_md *ctx) {
    void *data_end = (void *)(long)ctx->data_end;
    void *data = (void *)(long)ctx->data;

    // Boundary check for ethernet header
    struct ethhdr *eth = data;
    if ((void *)(eth + 1) > data_end) {
        return XDP_PASS;
    }

    // Only process IPv4 packets
    if (eth->h_proto != bpf_htons(ETH_P_IP)) {
        return XDP_PASS;
    }

    // Boundary check for IP header
    struct iphdr *ip = data + sizeof(*eth);
    if ((void *)(ip + 1) > data_end) {
        return XDP_PASS;
    }

    // Extract the source IP address
    __u32 src_ip = ip->saddr;

    // Check if the source IP is in our BLOCKED_IPS map
    __u32 *blocked = bpf_map_lookup_elem(&BLOCKED_IPS, &src_ip);
    if (blocked && *blocked == 1) {
        // Drop the packet entirely in kernel space before it reaches Tokio
        return XDP_DROP;
    }

    return XDP_PASS;
}

char _license[] SEC("license") = "GPL";
