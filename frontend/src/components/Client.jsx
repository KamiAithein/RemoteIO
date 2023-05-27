import { useState, useEffect } from "react";
import React, { Component } from 'react';
import PropTypes from 'prop-types';

class Client extends Component {
    static propTypes = {
        clientConnections: PropTypes.instanceOf(Array).isRequired,
        clientDevices: PropTypes.instanceOf(Array).isRequired,
        connectClient: PropTypes.func.isRequired,
        clientDisconnectClient: PropTypes.func.isRequired,
        changeClientInputDevice: PropTypes.func.isRequired
    };

    constructor(props) {
        super(props);

        this.state = {
            activeClientText: 'ws://0.0.0.0:8000',
        };
    }



    render() {
        const {
            props: {
                clientConnections,
                clientDevices,
                connectClient,
                clientDisconnectClient,
                changeClientInputDevice
            },
        } = this;

        let setActiveClientText = (text) => { this.setState({ activeClientText: text }) }


        return (
            <div>
                <div>
                    <input onChange={(e) => setActiveClientText(e.target.value)} value={this.state.activeClientText}></input>
                    <button onClick={() => connectClient(this.state.activeClientText)}>connect</button>
                </div>

                <ul>
                    {
                        clientConnections.map((client, ci) => (
                            <li>
                                <button>
                                    {client}
                                </button>
                                <button onClick={() => clientDisconnectClient(ci)}>
                                    disconnect
                                </button>
                                <ul>
                                    {
                                        clientDevices.map((device) => (
                                            <li>
                                                <button onClick={() => changeClientInputDevice(ci, device)}>
                                                    {device}
                                                </button>
                                            </li>
                                        ))
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

export default Client;