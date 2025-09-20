import { useCallback, useEffect, useState } from 'react'
import './App.css'

type ChoreResponse = {
  id: number,
  chore_name: string,
  frequency: number | null,
  last_completed_at: number | null,
}

type Chore = {
  id: number,
  chore_name: string,
  frequency: number | null,
  last_completed_at: number | null,
  overdue: boolean;
  daysUntilOverdue: string | null;
}

function processChore(chore: ChoreResponse): Chore {
  let isOverdueByFrequency = false;
  let isOverdueByIncompletion = false;
  let daysSinceLastComplete = null;


  if (chore.last_completed_at && chore.frequency) {
    daysSinceLastComplete = ((Date.now() / 1000) - chore.last_completed_at) / (60 * 60 * 24);
    console.log('days since last complete for ', chore.chore_name, daysSinceLastComplete);
    console.log('frequency for ', chore.chore_name, chore.frequency);
    isOverdueByFrequency = daysSinceLastComplete > chore.frequency / 24;
  }

  if (!chore.last_completed_at) {
    isOverdueByIncompletion = true;
  }

  const overdue = isOverdueByFrequency || isOverdueByIncompletion;
  return {
    ...chore,
    overdue,
    daysUntilOverdue: daysSinceLastComplete && !overdue ? ((chore.frequency! / 24) - daysSinceLastComplete).toFixed(2) : null,
  };
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
      setChores(chores.map(processChore));
    }
    getChores();
  }, [])

  const markChore = useCallback(async (chore: Chore) => {
    const response = await fetch(`/${chore.id}/mark-complete`, {
      method: "POST",
      body: JSON.stringify({
        clear_ticket: chore.overdue ? false : true // If ticket isn't overdue, clear last_completed_at to make it overdue
      }),
      headers: {
        "Content-Type": "application/json"
      }
    })
    if (!response.ok) {
      throw new Error('Network response was not ok');
    }
    const { chores } = await response.json();
    setChores(chores.map(processChore));
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
            <div className="days-left">{chore.daysUntilOverdue ? `Due in ${chore.daysUntilOverdue} days` : 'No deadline'}</div>
          </div>
        </button>
      )}
    </div >
  )
}

export default App
