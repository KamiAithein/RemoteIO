import { useState, useEffect } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

import Tabs from "./components/Tabs"

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
  const [activeClientText, setActiveClientText] = useState("ws://0.0.0.0:8000")

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

  async function clientDisconnectClient(client) {
    return await invoke("client_disconnect_client", { client });

  }

  async function changeServerOutputDevice(dname) {
    return await invoke("change_server_output_device", { dname });
  }

  // clientInputList.map((item, index) => (
//     <div>
//     <button onClick={() => changeClientInputDevice(item)}>{item}</button>
//   </div>
// ))}
  // async function updateClientInputList(name) {
  //   let list = await invoke("client_input_device_list", {name});

  //   let oldClientInputList = clientInputList;
  //   oldClientInputList[name] = list;
    
  //   setClientInputList(oldClientInputList);
  // }

  // async function getClientList() {
  //   let list = await invoke("client_list");
  //   console.log(list);
  //   return list;
  // }

  // async function updateServerOutputList() {
  //   let list = await invoke("server_output_device_list");
  //   setServerOutputList(list);
  // }

  // async function updateClientServerList() {
  //   let list = await invoke("client_server_list");
  //   setClientServerList(list);
  // }

  // async function updateServerClientList() {
  //   let list = await invoke("server_client_list");
  //   setServerClientList(list);
  // }

  // async function changeServerOutputDevice(dname) {
  //   await invoke("change_server_output_device", { dname })
  // }

  // async function changeClientInputDevice(cname, dname) {
  //   await invoke("change_client_input_device", { cname, dname })
  // }

  // async function clientDisconnectServer(name) {
  //   await invoke("client_disconnect_server", { name });
  //   await updateClientServerList();
  // }

  // async function clientConnectServer(wsAddr) {
  //   await invoke("client_connect_server", { wsAddr });
  //   await updateClientServerList();
  // }

  // async function handleWSAddrChange(event) {
  //   setWsAddr(event.target.value);
  // }

  // async function handleWSAddrConnect() {
  //   await clientConnectServer(wsAddr);
  //   await updateClientServerList();
  // }

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
          <ul>
            {
              serverConnections.map((conn) => (
                <li>
                  <button>
                    {conn}
                  </button>
                  <ul>
                    {
                      serverDevices.map((device) => (
                      <li>
                        <button onClick = {() => changeServerOutputDevice(device)}>
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
        <div label="Client">
          {/*create a client*/}
          <div>
            <form onSubmit = {() => connectClient(activeClientText)}>
              <input onChange = {(e) => setActiveClientText(e.target.value)} value = {activeClientText}></input>
              <button type = 'submit'>connect</button>
            </form>
          </div>

          <ul>
            {
              clientConnections.map((client) => (
                <li>
                  <button>
                    {client}
                  </button>
                  <button onClick = {() => clientDisconnectClient(client)}>
                    disconnect
                  </button>
                  <ul>
                    {
                      clientDevices.map((device) => (<li><button>{device}</button></li>))
                    }
                  </ul>
                </li>
              ))
            }
          </ul>
        </div>
      </Tabs>
    </div>
  );
}

export default App;
