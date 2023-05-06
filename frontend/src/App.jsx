import { useState, useEffect } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

import Tabs from "./components/Tabs"

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [list, setList] = useState(["default"]);
  const [wsAddr, setWsAddr] = useState("");

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    setGreetMsg(await invoke("greet", { name }));
  }

  async function getList() {
    let list = await invoke("list");
    setList(list);
  }

  async function pop(name) {
    await invoke("pop", { name });
    await getList();
  }

  async function connect(wsAddr) {
    await invoke("connect", { wsAddr });
    await getList();
  }

  useEffect(() => {
    getList()

    const intervalId = setInterval(() => {
      getList();
    }, 5000);

    return () => clearInterval(intervalId);
  }, []);

  const handleChange = (event) => {
    setWsAddr(event.target.value);
  }

  const handleClick = async () => {
    await connect(wsAddr);
  }

  return (
    <div>
      <h1>Welcome to RemoteIO!</h1>
      <Tabs>
        <div label="Server">
          {list.map((item, index) => (
            <div>
              <button>{item}</button>
              <button onClick={(e)=>{
                pop(item);
              }}>remove</button>
            </div>
          ))}
        </div>
        <div label="Client">
          <div>
              <input
                type="text"
                onChange={handleChange}
                value={wsAddr}
                ></input>

              <button onClick={handleClick}>connect!</button>
          </div>
        </div>
      </Tabs>
    </div>
  );
}

export default App;
