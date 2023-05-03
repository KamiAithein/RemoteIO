import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [list, setList] = useState("");

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    setGreetMsg(await invoke("greet", { name }));
  }

  async function getList() {
    setList(await invoke("list"));
  }

  return (
    <div className="container">
      <h1>Welcome to RemoteIO!</h1>
      <button class="submit" onClick={(e) => getList()}>Hello</button>
      <p>{list}</p>
    </div>
  );
}

export default App;
