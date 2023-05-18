import { useState, useEffect } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

import Tabs from "./components/Tabs"

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [serverClientList, setServerClientList] = useState(["default"]);
  const [clientServerList, setClientServerList] = useState(["default"]);
  const [serverOutputList, setServerOutputList] = useState(["default"]);
  const [clientInputList , setClientInputList ] = useState({"default": ["default"]});   
  const [wsAddr, setWsAddr] = useState("");

  // clientInputList.map((item, index) => (
//     <div>
//     <button onClick={() => changeClientInputDevice(item)}>{item}</button>
//   </div>
// ))}

  async function updateClientInputList(name) {
    let list = await invoke("client_input_device_list", {name});

    let oldClientInputList = clientInputList;
    oldClientInputList[name] = list;
    
    setClientInputList(oldClientInputList);
  }

  async function getClientList() {
    let list = await invoke("client_list");
    console.log(list);
    return list;
  }

  async function updateServerOutputList() {
    let list = await invoke("server_output_device_list");
    setServerOutputList(list);
  }

  async function updateClientServerList() {
    let list = await invoke("client_server_list");
    setClientServerList(list);
  }

  async function updateServerClientList() {
    let list = await invoke("server_client_list");
    setServerClientList(list);
  }

  async function changeServerOutputDevice(dname) {
    await invoke("change_server_output_device", { dname })
  }

  async function changeClientInputDevice(cname, dname) {
    await invoke("change_client_input_device", { cname, dname })
  }

  async function clientDisconnectServer(name) {
    await invoke("client_disconnect_server", { name });
    await updateClientServerList();
  }

  async function clientConnectServer(wsAddr) {
    await invoke("client_connect_server", { wsAddr });
    await updateClientServerList();
  }

  async function handleWSAddrChange(event) {
    setWsAddr(event.target.value);
  }

  async function handleWSAddrConnect() {
    await clientConnectServer(wsAddr);
    await updateClientServerList();
  }

  useEffect(() => {
    updateClientServerList();
    updateServerClientList();
    updateServerOutputList();
    
    getClientList().then((res) => res.forEach(clientName => {
      updateClientInputList(clientName)
    }))

    const intervalId = setInterval(() => {
      updateClientServerList();
      updateServerClientList();
      updateServerOutputList();

      getClientList().then((res) => res.forEach(clientName => {
        updateClientInputList(clientName)
      }))
    }, 5000);

    return () => clearInterval(intervalId);
  }, []);



  return (
    <div>
      <h1>Welcome to RemoteIO!</h1>
      <Tabs>
        <div label="Server">
          {serverClientList.map((item, index) => (
            <div>
              <button>{item}</button>
            </div>
          ))}
          {serverOutputList.map((item, index) => (
            <div>
              <button onClick={() => changeServerOutputDevice(item)}>{item}</button>
            </div>
          ))}
        </div>
        <div label="Client">
          <div>
            <input
              type="text"
              onChange={handleWSAddrChange}
              value={wsAddr}
            ></input>

            <button onClick={handleWSAddrConnect}>connect!</button>
          </div>
          <div>
            {clientServerList.map((item, index) => (
              <div>
                <button>
                  {item}
                </button>
                <button
                  onClick={() => clientDisconnectServer(item)}>
                  disconnect
                </button>
              </div>
            ))}
            {
              Object.entries(clientInputList).map(([client, devices], index) =>(
                <div>
                  <div>
                    {client}
                  </div>
                  <div>
                    {devices.map((item, index) => (
                      <button
                        onClick={() => changeClientInputDevice(client, item)}>
                        {item}
                    </button>
                    ))}
                  </div>
                </div>
              ))
            }
          </div>
        </div>
      </Tabs>
    </div>
  );
}

export default App;
