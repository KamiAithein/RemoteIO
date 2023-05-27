import { useState, useEffect } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

import Tabs from "./components/Tabs"
import Server from "./components/Server"
import Client from "./components/Client"

function App() {
  // const [greetMsg, setGreetMsg] = useState("");
  // const [name, setName] = useState("");
  // const [serverClientList, setServerClientList] = useState(["default"]);
  // const [clientServerList, setClientServerList] = useState(["default"]);
  // const [serverOutputList, setServerOutputList] = useState(["default"]);
  // const [clientInputList , setClientInputList ] = useState({"default": ["default"]});   
  // const [wsAddr, setWsAddr] = useState("");

  const [serverConnections, setServerConnections] = useState(["i1, i2, i3"]);
  const [serverDevices, setServerDevices] = useState(["i1, i2, i3"]);
  const [clientConnections, setClientConnections] = useState(["i1, i2, i3"]);
  const [clientDevices, setClientDevices] = useState(["i1, i2, i3"]);

  async function getServerConnections() {
    return await invoke("get_server_connections");
  }

  async function getServerDevices() {
    return await invoke("get_server_devices");
  }

  async function getClientConnections() {
    return await invoke("get_client_connections");

  }

  async function getClientDevices() {
    return await invoke("get_client_devices");
  }

  async function connectClient(address) {
    return await invoke("connect_client", { address });

  }

  async function clientDisconnectClient(cpos) {
    return await invoke("client_disconnect_client", { cpos });

  }

  async function changeServerOutputDevice(cpos, dname) {
    return await invoke("change_server_output_device", { cpos, dname });
  }

  async function changeClientInputDevice(cpos, dname) {
    return await invoke("change_client_input_device", { cpos, dname });
  }


  useEffect(() => {
    async function update() {
      setServerConnections(await getServerConnections());
      setServerDevices(await getServerDevices());
      setClientConnections(await getClientConnections());
      setClientDevices(await getClientDevices());
    }

    update();


    const intervalId = setInterval(() => {
      update();
    }, 1000);

    return () => clearInterval(intervalId);
  }, []);



  return (
    <div>
      <h1>Welcome to RemoteIO!</h1>
      <Tabs>

        <div label="Server">
          <Server
            serverConnections={serverConnections}
            serverDevices={serverDevices}>
          </Server>
        </div>

        <div label="Client">

          <Client
            clientConnections={clientConnections}
            clientDevices={clientDevices}
            connectClient={connectClient}
            clientDisconnectClient={clientDisconnectClient}
            changeClientInputDevice={changeClientInputDevice}>
          </Client>

        </div>
      </Tabs>
    </div>
  );
}

export default App;
