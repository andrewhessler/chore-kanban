import { useEffect, useState } from 'react'
import './App.css'

type Chore = {
  id: number,
  display_name: string,
  frequency: number,
  last_completed_at: number,
}

function App() {
  const [chores, setChores] = useState<Chore[]>([]);

  useEffect(() => {
    async function getChores() {
      const response = await fetch("/get-chores");
      if (!response.ok) {
        throw new Error('Network response was not ok');
      }
      const chores = await response.json();
      setChores(chores);

    }
    getChores();
  }, [])

  return (
    <>
      {chores.map((chore) => {
        <div>
          {chore.display_name}
        </div>
      })}
    </>
  )
}

export default App
