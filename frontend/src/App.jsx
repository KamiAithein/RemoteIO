import { useState, useEffect } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

import Tabs from "./components/Tabs"

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [list, setList] = useState(["default"]);

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
  }

  useEffect(() => {
    getList()

    const intervalId = setInterval(() => {
      getList();
    }, 5000);

    return () => clearInterval(intervalId);
  }, []);

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
          After 'while, Crocodile
        </div>
      </Tabs>
    </div>
  );
}

export default App;
