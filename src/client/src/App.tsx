import { useCallback, useEffect, useState } from 'react'
import './App.css'

type Chore = {
  id: number,
  chore_name: string,
  frequency: number | null,
  last_completed_at: number | null,
  overdue: boolean;
  days_until_overdue: number | null;
}

function App() {
  const [chores, setChores] = useState<Chore[]>([]);

  useEffect(() => {
    async function getChores() {
      const response = await fetch("/get-chores");
      if (!response.ok) {
        throw new Error('Network response was not ok');
      }
      const { chores } = await response.json();
      setChores(chores);
    }
    getChores();
  }, [])

  const markChore = useCallback(async (chore: Chore) => {
    const response = await fetch(`/${chore.id}/toggle-chore`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json"
      }
    })
    if (!response.ok) {
      throw new Error('Network response was not ok');
    }
    const { chores } = await response.json();
    setChores(chores);
  }, [])

  return (
    <div id="chores">
      <h2>Overdue</h2>
      {chores.filter((chore) => chore.overdue).map((chore) =>
        <button className="chore" onClick={() => markChore(chore)}>
          <div className="chore-card">
            <div className="chore-name">{chore.chore_name}</div>
          </div>
        </button>
      )}
      <h2>Upcoming</h2>
      {chores.filter((chore) => !chore.overdue).map((chore) =>
        <button className="chore" onClick={() => markChore(chore)}>
          <div className="chore-card">
            <div className="chore-name">{chore.chore_name}</div>
            <div className="days-left">{chore.days_until_overdue ? `${chore.days_until_overdue.toFixed(2)} days` : 'No deadline'}</div>
          </div>
        </button>
      )}
    </div >
  )
}

export default App
