/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

const {readFile} = require('fs').promises;
const {AgentConnector} = require('./agent_connector');

const main = async () => {
    
    const certs_dir = '/usr/share/smartm/certs/continuousc-demo';
    const pkg_dir = '/home/mdp/continuousc/mps';
    
    const broker = new AgentConnector({
	port: 9994,
	ca_path: certs_dir + '/ca.crt',
	key_path: certs_dir + '/backend.key',
	cert_path: certs_dir + '/backend.crt',
	verbose: true,
	ssh_conns: {
	    
	}
    });

    console.log('Connecting...');
    broker.on('agent-connected', (ev) => console.log('agent-connected', ev));
    await broker.connect();
    console.log('Connected!');

    try {
	//await broker.ping("350c0d69-d2d6-4366-8948-7a03f242884d");
	const status = await broker.get_connected_agents();
	console.log(JSON.stringify(status, undefined, 4));
    } catch (e) {
	console.log(`Error: ${e}`);
    }

    await broker.disconnect();
    console.log('Disconnected!');

}

main() //.catch(e => console.log(`Error: ${e}`))
