import React, { Component } from 'react';
import PropTypes from 'prop-types';

class Server extends Component {
    static propTypes = {
        serverConnections: PropTypes.instanceOf(Array).isRequired,
        serverDevices: PropTypes.instanceOf(Array).isRequired
    };



    render() {
        const {
            props: {
                serverConnections,
                serverDevices
            },
        } = this;

        return (
            <div>
                <ul>
                    {
                        serverConnections.map((conn, cpos) => (
                            <li>
                                <button>
                                    {conn}
                                </button>
                                <ul>
                                    {
                                        serverDevices.map((device) => (
                                            <li>
                                                <button onClick={() => changeServerOutputDevice(cpos, device)}>
                                                    {device}
                                                </button>
                                            </li>))
                                    }
                                </ul>
                            </li>
                        ))
                    }
                </ul>
            </div>
        )
    }
}

export default Server;