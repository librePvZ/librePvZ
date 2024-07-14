searchState.loadedDescShard("bevy_state", 0, "In Bevy, states are app-wide interdependent, finite state …\nProvides <code>App</code> and <code>SubApp</code> with state installation methods\nProvides definitions for the runtime conditions that …\nMost commonly used re-exported types.\nProvides definitions for the basic traits required by the …\nProvides [<code>StateScoped</code>] and [<code>clear_state_scoped_entities</code>] …\nState installation methods for <code>App</code> and <code>SubApp</code>.\nRegisters the <code>StateTransition</code> schedule in the …\nSets up a type implementing <code>ComputedStates</code>.\nSets up a type implementing <code>SubStates</code>.\nEnable state-scoped entity clearing for state <code>S</code>.\nReturns the argument unchanged.\nInitializes a <code>State</code> with standard starting values.\nInserts a specific <code>State</code> to the current <code>App</code> and overrides …\nCalls <code>U::from(self)</code>.\nGenerates a <code>Condition</code>-satisfying closure that returns <code>true</code> …\nA <code>Condition</code>-satisfying system that returns <code>true</code> if the …\nA <code>Condition</code>-satisfying system that returns <code>true</code> if the …\nA state whose value is automatically computed based on the …\nHow many other states this state depends on. Used to help …\nHow many other states this state depends on. Used to help …\nSystem set that runs enter schedule(s) for state <code>S</code>.\nSystem set that runs exit schedule(s) for state <code>S</code>.\nThis trait allows a state to be mutated directly using the …\nThe next state of <code>State&lt;S&gt;</code>.\nThe label of a <code>Schedule</code> that <strong>only</strong> runs whenever <code>State&lt;S&gt;</code> …\nThe label of a <code>Schedule</code> that <strong>only</strong> runs whenever <code>State&lt;S&gt;</code> …\nThe label of a <code>Schedule</code> that <strong>only</strong> runs whenever <code>State&lt;S&gt;</code> …\nThere is a pending transition for state <code>S</code>\nThe total <code>DEPENDENCY_DEPTH</code> of all the states that are part …\nThe set of states from which the <code>Self</code> is derived.\nThe set of states from which the <code>Self</code> is derived.\nA finite-state machine whose transitions have associated …\nA <code>States</code> type or tuple of types which implement <code>States</code>.\nRuns state transitions.\nEvent sent when any state transition of <code>S</code> happens. This …\nTypes that can define world-wide states in a finite-state …\nA sub-state is a state that exists only when the source …\nSystem set that runs transition schedule(s) for state <code>S</code>.\nNo state transition is pending\nComputes the next value of <code>State&lt;Self&gt;</code>. This function gets …\nThe state being entered.\nThe state being entered.\nThe state being exited.\nThe state being exited.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nReturns the argument unchanged.\nGet the current state.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nCalls <code>U::from(self)</code>.\nReturns the latest state transition event of type <code>S</code>, if …\nCreates a new state with a specific value.\nThis function sets up systems that compute the state …\nThis function sets up systems that compute the state …\nSets up the systems needed to compute <code>T</code> whenever any <code>State</code> …\nThis function registers all the necessary systems to apply …\nThis function registers all the necessary systems to apply …\nThis function sets up systems that compute the state …\nThis function sets up systems that compute the state …\nSets up the systems needed to compute whether <code>T</code> exists …\nRemove any pending changes to <code>State&lt;S&gt;</code>\nTentatively set a pending state transition to <code>Some(state)</code>.\nSets up the schedules and systems for handling state …\nThis function gets called whenever one of the <code>SourceStates</code> …\nEntities marked with this component will be removed when …\nRemoves entities marked with <code>StateScoped&lt;S&gt;</code> when their …\nReturns the argument unchanged.\nCalls <code>U::from(self)</code>.")