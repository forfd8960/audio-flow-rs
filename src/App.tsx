import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import './App.css'

interface RecordingState {
  isRecording: boolean
  state: string
  volume: number
  transcription: string
}

interface Config {
  apiKey: string
  language: string
  listenKey: string
}

function App() {
  const [recording, setRecording] = useState<RecordingState>({
    isRecording: false,
    state: 'idle',
    volume: 0,
    transcription: ''
  })

  const [config, setConfig] = useState<Config>({
    apiKey: '',
    language: 'zh-CN',
    listenKey: 'f13'
  })

  const [connected, setConnected] = useState(false)

  useEffect(() => {
    // Listen for events from Rust backend
    const unlisten1 = invoke('listen_recording_state', (event: any) => {
      setRecording(prev => ({
        ...prev,
        isRecording: event.isRecording,
        state: event.state
      }))
    })

    const unlisten2 = invoke('listen_volume_level', (event: any) => {
      setRecording(prev => ({
        ...prev,
        volume: event.level
      }))
    })

    const unlisten3 = invoke('listen_transcription', (event: any) => {
      setRecording(prev => ({
        ...prev,
        transcription: event.isFinal
          ? prev.transcription + event.text
          : prev.transcription + event.text
      }))
    })

    return () => {
      unlisten1.then(f => f())
      unlisten2.then(f => f())
      unlisten3.then(f => f())
    }
  }, [])

  const toggleRecording = async () => {
    if (recording.isRecording) {
      await invoke('stop_capture')
      setConnected(false)
    } else {
      await invoke('start_capture', { apiKey: config.apiKey })
      setConnected(true)
    }
  }

  const clearTranscription = () => {
    setRecording(prev => ({ ...prev, transcription: '' }))
  }

  const copyTranscription = () => {
    navigator.clipboard.writeText(recording.transcription)
  }

  return (
    <div className="app">
      <header className="header">
        <h1>🎙️ AudioFlow</h1>
        <p>Real-time Speech-to-Text Transcription</p>
      </header>

      <main className="main">
        {/* Recording Controls */}
        <section className="card recording-card">
          <div className="recording-status">
            <div className={`status-indicator ${recording.isRecording ? 'recording' : ''}`}>
              <span className="pulse"></span>
            </div>
            <div className="status-text">
              <h2>{recording.isRecording ? 'Recording...' : 'Ready'}</h2>
              <p>{recording.state}</p>
            </div>
          </div>

          <div className="volume-meter">
            <div
              className="volume-fill"
              style={{ width: `${recording.volume * 100}%` }}
            ></div>
          </div>

          <button
            className={`record-btn ${recording.isRecording ? 'stop' : 'start'}`}
            onClick={toggleRecording}
          >
            {recording.isRecording ? '⏹️ Stop' : '🎤 Start Recording'}
          </button>
        </section>

        {/* Settings */}
        <section className="card settings-card">
          <h3>⚙️ Settings</h3>

          <div className="form-group">
            <label>ElevenLabs API Key</label>
            <input
              type="password"
              placeholder="Enter your API key"
              value={config.apiKey}
              onChange={e => setConfig(prev => ({ ...prev, apiKey: e.target.value }))}
            />
          </div>

          <div className="form-group">
            <label>Language</label>
            <select
              value={config.language}
              onChange={e => setConfig(prev => ({ ...prev, language: e.target.value }))}
            >
              <option value="zh-CN">Chinese (Simplified)</option>
              <option value="zh-TW">Chinese (Traditional)</option>
              <option value="en">English</option>
              <option value="ja">Japanese</option>
              <option value="ko">Korean</option>
            </select>
          </div>

          <div className="form-group">
            <label>Listen Key</label>
            <input
              type="text"
              value={config.listenKey}
              onChange={e => setConfig(prev => ({ ...prev, listenKey: e.target.value }))}
            />
          </div>

          <button className="save-btn" onClick={() => invoke('save_config', config)}>
            Save Settings
          </button>
        </section>

        {/* Transcription */}
        <section className="card transcription-card">
          <div className="transcription-header">
            <h3>📝 Transcription</h3>
            <div className="transcription-actions">
              <button onClick={clearTranscription}>Clear</button>
              <button onClick={copyTranscription}>Copy</button>
            </div>
          </div>

          <div className="transcription-content">
            {recording.transcription || (
              <p className="placeholder">Transcribed text will appear here...</p>
            )}
            {recording.transcription && (
              <pre>{recording.transcription}</pre>
            )}
          </div>

          {connected && (
            <div className="connection-status">
              <span className="dot"></span>
              Connected to ElevenLabs Scribe
            </div>
          )}
        </section>
      </main>

      <footer className="footer">
        <p>Press {config.listenKey} to toggle recording</p>
        <p>AudioFlow v0.1.0</p>
      </footer>
    </div>
  )
}

export default App
