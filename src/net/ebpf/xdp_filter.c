#include <bpf/bpf_helpers.h>
#include <linux/bpf.h>
#include <linux/if_ether.h>
#include <linux/in.h>
#include <linux/ip.h>
#include <linux/types.h>
#include <linux/udp.h>

#define PHONECAM_PORT 9999
#define PHONECAM_MAGIC_0 0x50 // 'P'
#define PHONECAM_MAGIC_1 0x43 // 'C'

SEC("xdp")
int xdp_phonecam_filter(struct xdp_md *ctx) {
  void *data_end = (void *)(long)ctx->data_end;
  void *data = (void *)(long)ctx->data;

  // Parse Ethernet
  struct ethhdr *eth = data;
  if ((void *)(eth + 1) > data_end)
    return XDP_PASS;

  if (eth->h_proto != __constant_htons(ETH_P_IP))
    return XDP_PASS;

  // Parse IP
  struct iphdr *ip = (void *)(eth + 1);
  if ((void *)(ip + 1) > data_end)
    return XDP_PASS;

  if (ip->protocol != IPPROTO_UDP)
    return XDP_PASS;

  // Parse UDP
  struct udphdr *udp = (void *)(ip + 1);
  if ((void *)(udp + 1) > data_end)
    return XDP_PASS;

  if (udp->dest != __constant_htons(PHONECAM_PORT))
    return XDP_PASS;

  // Check PhoneCam magic
  __u8 *payload = (void *)(udp + 1);
  if ((void *)(payload + 8) > data_end)
    return XDP_PASS;

  if (payload[0] == PHONECAM_MAGIC_0 && payload[1] == PHONECAM_MAGIC_1) {
    // High priority packet detected
    // In a real extreme scenario, we could use XDP_REDIRECT to a specific core
    return XDP_PASS;
  }

  return XDP_PASS;
}

char _license[] SEC("license") = "GPL";
