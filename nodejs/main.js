/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

const fs = require("fs").promises;
var glob = require("glob")
const { metric_schemas } = require('smart-agent-lib');
const { AgentConnector, sleep } = require("./agent_connector");
const { DBConnector } = require("./db_connector");


const prot_config_si = {
    SNMP: {
	host_config: {
	    auth: {
		version: "2c",
		community: "SITRO",
	    },
	    bulk_host: true,
	    use_walk: false,
	    bulk_opts: {},
	    port: 161
	},
	host_name: "192.168.10.1",
	ip_addr: "192.168.10.1"
    }
};

const prot_config_stl = {
    SNMP: {
	host_name: "172.22.255.23",
	ip_addr: "172.22.255.23",
	host_config: {
	    auth: {
		level: "authNoPriv",
		version: "3",
		auth: {
		    password: "5fMCIxTtQuoIIhU2wUD0",
		    protocol: "md5",
		    user: "mnow_snmp"
		}
	    },
	    bulk_host: true,
	    use_walk: false,
	    bulk_opts: {
		max_size: 10,
		max_width: 1,
		max_length: 10
	    },
	    port: 161
	}
    }
};

const packages = [
    '/usr/local/share/continuousc/mps/Interface.json',
    //'/usr/local/share/continuousc/mps/Entity.json',
    '/usr/local/share/continuousc/mps/SNMPv2.json',
    '/usr/local/share/continuousc/mps/IP.json',
    '/usr/local/share/continuousc/mps/IP_forwarding.json',
    //'/usr/local/share/continuousc/mps/nping.json',
    '/usr/local/share/continuousc/mps/Bridge.json',
    '/usr/local/share/continuousc/mps/EDP.json',
    '/usr/local/share/continuousc/mps/CDP.json',
];

const interface_rules = [
  {
    "selector": {
      "always": null
    },
    "actions": [
      {
        "path": [],
        "action": {
          "override": {
            "config": {
            },
            "thresholds": {
              "ifInErrors": {
                "warning": null,
                "critical": null
              },
              "ifOutErrors": {
                "warning": null,
                "critical": null
              },
              "ifOperStatus": {
                "warning": {
                  "is_not": "up"
                },
                "critical": {
                  "is": "down"
                }
              },
              "ifHighSpeed": {
                "warning": {
                  "absolute": {
                    "ne": {
                      "value": 1,
                      "unit": {
                        "Bandwidth": {
                          "Information": {
                            "Bit": "Giga"
                          },
                          "Time": {
                            "Second": "Unit"
                          }
                        }
                      }
                    }
                  }
                },
                "critical": null
              },
              "ifOutDiscards": {
                "warning": null,
                "critical": null
              },
	      "ifHCInOctets": {
                "warning": null,
                "critical": null
              },
              "ifInDiscards": {
                "warning": {
                  "absolute": {
                    "ge": 123
                  }
                },
                "critical": {
                  "absolute": {
                    "gt": 125
                  }
                }
              }
            }
          }
        }
      }
    ]
  }
];

main().catch((err) => console.log("Exception: " + err));

async function main() {
    let instance = 'abo-dev'; //require("os").userInfo().username + '-dev';
    let db = new DBConnector();
    let conn = new AgentConnector(
	"127.0.0.1",
	{
	    ping: async (agent) => {
		console.log("\x1b[31mReceived ping from " + agent + "\x1b[0m");
	    },
	    data: async (agent, index, data) => {
		console.log('Received data from ' + agent + ' for index "' + index
			    + '": ' + JSON.stringify(data));
	    },
	},
	{
	    port: 9998,
	    ca_path: `/usr/share/smartm/certs/${instance}/ca.crt`,
	    key_path: `/usr/share/smartm/certs/${instance}/backend.key`,
	    cert_path: `/usr/share/smartm/certs/${instance}/backend.crt`,
	    ssh_conns: {
		'4e9e2e83-5080-4cc1-87ad-05434095485c': {
		    host: '127.0.0.1',
		    agent_port: 9997,
		    jump_hosts: ["root@mnvpn01", "root@192.168.240.77"],
		    known_hosts: {},
		    private_key: await fs.readFile("/home/abo/.ssh/id_rsa")
		}
	    }
	}
    );

    await db.connect();
    await db.wait_for_databases();

    for (file of packages) {
	let name = file.substring(file.lastIndexOf('/')+1,
				  file.lastIndexOf('.'));
    	console.log("Loading '" + name + "' package into dbdaemon...");
    	try {
	    const data = await fs.readFile(file);
    	    await db.load_package(name, '1.0.0', data.toString());
    	    console.log('Success!');
    	} catch (e) {
    	    console.log('Failed: ' + e);
    	}
    }

    console.log("Loading ifEntry ruleset");
    try {
    	await db.load_ruleset('MP/interface/ifEntry', interface_rules);
    	console.log('Success!');
    } catch (e) {
    	console.log('Failed: ' + e);
    }

    
    const metrics = metric_schemas(JSON.parse(await fs.readFile(
    	"/usr/local/share/continuousc/mps/Interface.json"
    )));

    for (entry of Object.entries(metrics)) {
    	const [index,definition] = entry;
    	// try { await db.unregister_table(index); }
    	// catch {}
    	await db.register_table(index, definition);
    }
    console.log("Schemas registered!");

    conn.on("agent-disconnected", (agent) =>
	    console.log("Disconnect from " + agent)
	   );
    
    conn.on("agent-connected", async (agent) => {

	console.log("Connection from " + agent)

	// try {
	//     let res = await conn.port_scan(agent, ['mndev02', 'mnvmdev02'],
	// 				   ["80/tcp", "443/tcp", "161/udp"]);
	//     console.log(JSON.stringify(res, undefined, 4));
	// } catch (e) {
	//     console.log('Error: ' + e);
	// }
	
    	//glob('/usr/local/share/continuousc/mps/*.json', {}, async (er, files) => {
    	for (file of packages) {
    	    let name = file.substring(file.lastIndexOf('/')+1,
				      file.lastIndexOf('.'));
    	    console.log("Loading '" + name + "' package into agent...");
    	    try {
    		await conn.load_pkg(agent, name, '1.0.0',
				    await fs.readFile(file));
    		console.log('Success!');
    	    } catch (e) {
    		console.log('Failed: ' + e);
    	    }
    	}
    	//});

	conn.config(agent, {
	    tasks: [
		{
		    task: {
			checks: {
			    host_id: "172.22.254.61",
			    mp_id: "Interface",
			    //table_ids: ["Interface_Status_ifEntry"],
			    config: prot_config_stl
			}
		    },
		    schedule: {
			period: 5
		    }
		}
	    ]
	});

	try {
	    for (const table_ids of [['EDP_Vlans_extremeEdpNeighborEntry',
				      'Bridge_Base Ports_BasePortEntry',
				      ]]) {
		res = await conn.get_etc_tables(agent, table_ids, prot_config_stl, "discovery");
		console.log(JSON.stringify(res));
	    }
	} catch (e) {
	    console.log("Error: " + e.toString());
	}

    });

    conn.connect();

    // while (true) {

    // 	await sleep(3000);

    // 	try {
	    
    // 	    let metrics = await db.read_item_metrics(
    // 	    	"snmpv2","host",
    // 	    	"172.22.254.61");

    // 	    console.log(JSON.stringify(metrics, undefined, 4));

    // 	    console.log("Metrics:" + Object.entries(metrics).map(
    // 		([table, metrics]) => "\n- " + table + ": "
    // 		    + metrics.value.result.success.metrics.length
    // 		    + " row(s)" + " (timestamp: " + metrics.timestamp
    // 		    + ")").join(""));

    // 	} catch (err) {
    // 	    console.log("Failed to retrieve metrics: " + err)
    // 	}

    // }

}
