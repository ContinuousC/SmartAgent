/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

const { CborRpcClient, RpcError } = require('rpc');
const {EventEmitter} = require("events");


/* An AgentConnector manages the connection to the agent server,
 * retrying at regular intervals if the connection fails. 
 */
class AgentConnector extends CborRpcClient {

    constructor(opts) {
	super({handle_messages: false, ...opts});
	this.broker_req_id = 0;

        this.on('message', msg => {
            try {
		if ('broker' in msg) {
		    const {message} = msg.broker;
                    const {req_id, response} = message;
                    this.emit('broker-response-' + req_id, response);
		} else if ('agent' in msg) {
		    const {agent_id, message} = msg.agent;
                    const {req_id, response} = message;
                    this.emit('response-' + req_id, [agent_id, response]);
		} else if ('broker-event' in msg) {
		    const [event,args] = Object.entries(msg['broker-event'].event)[0];
		    this.emit(event, args);
		}
            } catch (e) {
                console.log("Warning: invalid message: " + e);
            }
        });
    }
    
    async broker_request(request, options, ctx = {}) {

        if (!this.connected) {
            throw new RpcError("Attempt to write in disconnected state!");
        }

        let req_id = this.broker_req_id++;
        let ac = new AbortController();
        let timeout = this.timeout(ctx.timeout || this.request_timeout, ac);
        let response = this.broker_response(req_id, ac);

        await this.write_msg({
	    broker: {
		message: {
		    req_id,
		    request: { [request]: options }
		}
	    }
	});
        let res = await Promise.race([response, timeout]);

        return res;

    }
    
    async request(agent_id, request, options, ctx = {}) {

        if (!this.connected) {
            throw new RpcError("Attempt to write in disconnected state!");
        }

        let req_id = this.req_id++;
        let ac = new AbortController();
        let timeout = this.timeout(ctx.timeout || this.request_timeout, ac);
        let response = this.response(agent_id, req_id, ac);

        await this.write_msg({
	    agent: {
		agent_id,
		message: {
		    req_id,
		    request: { [request]: options }
		}
	    }
	});
        let res = await Promise.race([response, timeout]);

        return res;

    }

    broker_response(req_id, ac) {
        return new Promise((resolve,reject) => {
            let listener = res => {
                if (ac && ac.aborted) {
                    //console.log('Debug: ignoring late response for ' + req_id);
                } else {
                    if ('Ok' in res) {
                        resolve(res.Ok);
                    } else if ('Err' in res) {
                        reject(new RpcError(res.Err))
                    } else {
                        console.log(JSON.stringify(res))
                        reject(`rpc call failed without an error message`)
                    }
                    if (ac) ac.abort();
                }
            };
            this.once('broker-response-' + req_id, listener);
            if (ac) ac.signal.once('abort', ev => {
                super.off('broker-response-' + req_id, listener);
                reject(ev);
            });
        });
    }

    response(agent_id, req_id, ac) {
        return new Promise((resolve,reject) => {
            let listener = ([id, res]) => {
                if (ac && ac.aborted) {
                    //console.log('Debug: ignoring late response for ' + req_id);
                } else {
		    if (id !== agent_id) {
			reject('received response from wrong agent: ' + id.toString());
		    } else if ('Ok' in res) {
                        resolve(res.Ok);
                    } else if ('Err' in res) {
                        reject(new RpcError(res.Err))
                    } else {
                        console.log(JSON.stringify(res))
                        reject(`rpc call failed without an error message`)
                    }
                    if (ac) ac.abort();
                }
            };
            this.once('response-' + req_id, listener);
            if (ac) ac.signal.once('abort', ev => {
                super.off('response-' + req_id, listener);
                reject(ev);
            });
        });
    }

    timeout(ms, ac) {
        return new Promise((resolve,reject) => {
            let timeout = setTimeout(() => {
                reject("timeout");
                if (ac) ac.abort();
            }, ms);
            if (ac) ac.signal.once('abort', ev => {
                clearTimeout(timeout);
                //reject("aborted");
            });
        });
    }
    
    {{BrokerService}}

    {{AgentService}}

}

/* No longer required after update to node >= 15 */

class AbortController {
    constructor() {
        this.aborted = false;
        this.signal = new EventEmitter();
    }
    abort() {
        if (!this.aborted) {
            this.aborted = true;
            this.signal.emit('abort');
        }
    }
}

module.exports = {
    AgentConnector
}
