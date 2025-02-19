/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::convert::TryInto;

use futures::stream::TryStreamExt;
use netlink_packet_route::rtnl;

use agent_api::{ArpEntry, IpRoute};

use super::error::Result;

pub async fn get_routes() -> Result<Vec<IpRoute>> {
    let (conn, handle, _) = rtnetlink::new_connection()?;
    tokio::spawn(conn);

    let mut response = handle.link().get().execute();
    let mut links = HashMap::new();
    let mut link_up = HashMap::new();

    while let Some(link) = response.try_next().await? {
        links.insert(
            link.header.index,
            link.nlas
                .iter()
                .filter_map(|x| match x {
                    rtnl::link::nlas::Nla::IfName(name) => Some(name),
                    _ => None,
                })
                .next()
                .map(String::as_ref)
                .unwrap_or("unknown")
                .to_string(),
        );
        link_up.insert(
            link.header.index,
            link.header.flags & rtnl::constants::IFF_LOWER_UP > 0,
        );
    }

    let mut response = handle.route().get(rtnetlink::IpVersion::V4).execute();
    let mut routes = Vec::new();

    while let Some(route) = response.try_next().await? {
        if route.header.address_family as u16 != rtnl::constants::AF_INET {
            continue;
        }

        let table = route
            .nlas
            .iter()
            .filter_map(|x| match x {
                rtnl::route::nlas::Nla::Table(n) => Some(*n),
                _ => None,
            })
            .next()
            .unwrap_or(route.header.table as u32);

        if table != rtnl::constants::RT_TABLE_MAIN as u32 {
            continue;
        }

        routes.push(IpRoute {
            dev: route
                .output_interface()
                .and_then(|n| links.get(&n))
                .map(String::as_ref)
                .unwrap_or("unknown")
                .to_string(),
            up: route
                .output_interface()
                .and_then(|n| link_up.get(&n))
                .cloned()
                .unwrap_or(false),
            proto: match route.header.protocol {
                rtnl::constants::RTPROT_RA => "ra",
                rtnl::constants::RTPROT_MRT => "mrt",
                rtnl::constants::RTPROT_NTK => "ntk",
                rtnl::constants::RTPROT_BIRD => "bird",
                rtnl::constants::RTPROT_BOOT => "boot",
                rtnl::constants::RTPROT_DHCP => "dhcp",
                rtnl::constants::RTPROT_XORP => "xorp",
                rtnl::constants::RTPROT_BABEL => "babel",
                rtnl::constants::RTPROT_GATED => "gated",
                rtnl::constants::RTPROT_ZEBRA => "zebra",
                rtnl::constants::RTPROT_KERNEL => "kernel",
                rtnl::constants::RTPROT_STATIC => "static",
                rtnl::constants::RTPROT_UNSPEC => "unspec",
                rtnl::constants::RTPROT_MROUTED => "mrouted",
                rtnl::constants::RTPROT_DNROUTED => "dnrouted",
                rtnl::constants::RTPROT_REDIRECT => "redirect",
                _ => "unknown",
            }
            .to_string(),
            scope: match route.header.scope {
                rtnl::constants::RT_SCOPE_HOST => "host",
                rtnl::constants::RT_SCOPE_LINK => "link",
                rtnl::constants::RT_SCOPE_SITE => "site",
                rtnl::constants::RT_SCOPE_NOWHERE => "nowhere",
                rtnl::constants::RT_SCOPE_UNIVERSE => "universe",
                _ => "unknown",
            }
            .to_string(),
            table: match table.try_into() {
                Ok(rtnl::constants::RT_TABLE_MAIN) => "main",
                Ok(rtnl::constants::RT_TABLE_LOCAL) => "local",
                Ok(rtnl::constants::RT_TABLE_COMPAT) => "compat",
                Ok(rtnl::constants::RT_TABLE_UNSPEC) => "unspec",
                Ok(rtnl::constants::RT_TABLE_DEFAULT) => "default",
                _ => "unknown",
            }
            .to_string(),
            via: route.gateway().map(|addr| format!("{}", addr)),
            src: route
                .source_prefix()
                .map(|(addr, prefix)| format!("{}/{}", addr, prefix)),
            dst: route
                .destination_prefix()
                .map(|(addr, prefix)| format!("{}/{}", addr, prefix)),
            metric: route
                .nlas
                .iter()
                .filter_map(|x| match x {
                    rtnl::route::nlas::Nla::Priority(ms) => Some(*ms),
                    _ => None,
                })
                .next(),
        });
    }

    Ok(routes)
}

pub async fn get_neighbours() -> Result<Vec<ArpEntry>> {
    let (conn, handle, _) = rtnetlink::new_connection()?;
    tokio::spawn(conn);

    let mut response = handle.link().get().execute();
    let mut links = HashMap::new();
    //let mut link_up = HashMap::new();

    while let Some(link) = response.try_next().await? {
        links.insert(
            link.header.index,
            link.nlas
                .iter()
                .filter_map(|x| match x {
                    rtnl::link::nlas::Nla::IfName(name) => Some(name),
                    _ => None,
                })
                .next()
                .map(String::as_ref)
                .unwrap_or("unknown")
                .to_string(),
        );
        //link_up.insert(link.header.index, link.header.flags
        //& rtnl::constants::IFF_LOWER_UP > 0);
    }

    let mut response = handle.neighbours().get().execute();
    let mut neighbours = Vec::new();

    while let Some(neighbour) = response.try_next().await? {
        if neighbour.header.family as u16 != rtnl::constants::AF_INET {
            continue;
        }

        if neighbour.header.ntype as u16 != rtnl::constants::NDA_DST {
            continue;
        }

        /*if neighbour.header.state == rtnl::constants::NUD_FAILED {
        continue;
        }*/

        neighbours.push(ArpEntry {
            ip: neighbour
                .nlas
                .iter()
                .filter_map(|x| match x {
                    rtnl::neighbour::nlas::Nla::Destination(addr) => addr
                        .to_vec()
                        .try_into()
                        .ok()
                        .map(|addr| format_ipv4(u32::from_be_bytes(addr))),
                    _ => None,
                })
                .next(),
            mac: neighbour
                .nlas
                .iter()
                .filter_map(|x| match x {
                    rtnl::neighbour::nlas::Nla::LinkLocalAddress(addr) => addr
                        .to_vec()
                        .try_into()
                        .ok()
                        .map(|addr| format_mac(&addr)),
                    _ => None,
                })
                .next(),
            vlan: neighbour
                .nlas
                .iter()
                .filter_map(|x| match x {
                    rtnl::neighbour::nlas::Nla::Vlan(n) => Some(*n),
                    _ => None,
                })
                .next(),
            dev: links
                .get(
                    &neighbour
                        .nlas
                        .iter()
                        .filter_map(|x| match x {
                            rtnl::neighbour::nlas::Nla::IfIndex(i) => Some(*i),
                            _ => None,
                        })
                        .next()
                        .unwrap_or(neighbour.header.ifindex),
                )
                .map(|x| x.to_string()),
            ntype: match neighbour.header.ntype as u16 {
                rtnl::constants::NDA_CACHEINFO => "cacheinfo",
                rtnl::constants::NDA_DST => "dst",
                rtnl::constants::NDA_IFINDEX => "ifindex",
                rtnl::constants::NDA_LINK_NETNSID => "link_netnsid",
                rtnl::constants::NDA_LLADDR => "lladdr",
                rtnl::constants::NDA_MASTER => "master",
                rtnl::constants::NDA_PORT => "port",
                rtnl::constants::NDA_PROBES => "probes",
                rtnl::constants::NDA_SRC_VNI => "src_vni",
                rtnl::constants::NDA_UNSPEC => "unspec",
                rtnl::constants::NDA_VLAN => "vlan",
                rtnl::constants::NDA_VNI => "vni",
                _ => "unknown",
            }
            .to_string(),
            state: match neighbour.header.state {
                rtnl::constants::NUD_DELAY => "delay",
                rtnl::constants::NUD_FAILED => "failed",
                rtnl::constants::NUD_INCOMPLETE => "incomplete",
                rtnl::constants::NUD_NOARP => "noarp",
                rtnl::constants::NUD_NONE => "none",
                rtnl::constants::NUD_PERMANENT => "permanent",
                rtnl::constants::NUD_PROBE => "probe",
                rtnl::constants::NUD_REACHABLE => "reachable",
                rtnl::constants::NUD_STALE => "stale",
                _ => "unknown",
            }
            .to_string(),
            flags: [
                (rtnl::constants::NTF_EXT_LEARNED, "ext_learned"),
                (rtnl::constants::NTF_MASTER, "master"),
                (rtnl::constants::NTF_OFFLOADED, "offloaded"),
                (rtnl::constants::NTF_PROXY, "proxy"),
                (rtnl::constants::NTF_ROUTER, "router"),
                (rtnl::constants::NTF_SELF, "self"),
                (rtnl::constants::NTF_USE, "use"),
            ]
            .iter()
            .filter_map(|(flag, name)| {
                (neighbour.header.flags & flag > 0).then(|| name.to_string())
            })
            .collect(),
        });
    }

    Ok(neighbours)
}

fn format_mac(addr: &[u8; 6]) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        addr[0], addr[1], addr[2], addr[3], addr[4], addr[5]
    )
}

fn format_ipv4(addr: u32) -> String {
    format!(
        "{}.{}.{}.{}",
        (addr >> 24) & 0xff,
        (addr >> 16) & 0xff,
        (addr >> 8) & 0xff,
        (addr) & 0xff,
    )
}
