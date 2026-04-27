"use client";

import React, { useState } from "react";
import { 
  Mic, 
  Translate, 
  Volume2, 
  Play, 
  Wand2, 
  Loader2, 
  CheckCircle2, 
  AlertCircle,
  FileAudio,
  ArrowRight
} from "lucide-react";

export default function VoicesLabPage() {
  const [audioUrl, setAudioUrl] = useState("");
  const [targetLang, setTargetLang] = useState("fr");
  const [status, setStatus] = useState<"idle" | "loading" | "success" | "error">("idle");
  const [logs, setLogs] = useState<string[]>([]);
  const [resultAudio, setResultAudio] = useState<string | null>(null);

  const addLog = (msg: string) => {
    setLogs(prev => [...prev, `${new Date().toLocaleTimeString()} - ${msg}`]);
  };

  const handleRunTest = async () => {
    if (!audioUrl) return;
    
    setStatus("loading");
    setLogs([]);
    setResultAudio(null);
    addLog("Starting Audio Lab Pipeline...");

    try {
      addLog(`Requesting orchestrator to process audio: ${audioUrl}`);
      
      const response = await fetch(`${process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000'}/api/voices-lab/test`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Authorization": `Bearer ${process.env.NEXT_PUBLIC_API_KEY || 'changeme'}`
        },
        body: JSON.stringify({
          audio_url: audioUrl,
          target_lang: targetLang
        })
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.error || "Failed to process audio");
      }

      const data = await response.json();
      addLog("✅ Pipeline finished successfully!");
      setResultAudio(data.result_url);
      setStatus("success");
    } catch (err: any) {
      setStatus("error");
      addLog(`❌ Error: ${err.message}`);
    }
  };

  return (
    <div className="p-8 space-y-8 animate-in fade-in duration-700">
      <div className="flex flex-col gap-2">
        <h1 className="text-4xl font-bold tracking-tight text-gradient">Voices Lab</h1>
        <p className="text-slate-400">Test the standalone audio pipeline: Transcription → Translation → Cloning → Composition.</p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
        {/* Configuration Card */}
        <div className="lg:col-span-1 space-y-6">
          <div className="glass p-6 rounded-2xl border border-white/5 space-y-6">
            <div className="flex items-center gap-2 text-primary font-semibold">
              <Wand2 className="w-5 h-5" />
              <span>Configuration</span>
            </div>

            <div className="space-y-4">
              <div className="space-y-2">
                <label className="text-sm font-medium text-slate-300">Source Audio URL</label>
                <div className="relative">
                  <FileAudio className="absolute left-3 top-3 w-5 h-5 text-slate-500" />
                  <input 
                    type="text" 
                    placeholder="https://s3.amazonaws.com/..." 
                    className="w-full bg-black/40 border border-white/10 rounded-xl py-2.5 pl-10 pr-4 text-sm focus:border-primary/50 outline-none transition-colors"
                    value={audioUrl}
                    onChange={(e) => setAudioUrl(e.target.value)}
                  />
                </div>
              </div>

              <div className="space-y-2">
                <label className="text-sm font-medium text-slate-300">Target Language</label>
                <select 
                  className="w-full bg-black/40 border border-white/10 rounded-xl py-2.5 px-4 text-sm focus:border-primary/50 outline-none transition-colors"
                  value={targetLang}
                  onChange={(e) => setTargetLang(e.target.value)}
                >
                  <option value="fr">French (FR)</option>
                  <option value="en">English (EN)</option>
                  <option value="es">Spanish (ES)</option>
                  <option value="de">German (DE)</option>
                  <option value="it">Italian (IT)</option>
                  <option value="ja">Japanese (JA)</option>
                </select>
              </div>

              <button 
                onClick={handleRunTest}
                disabled={status === "loading" || !audioUrl}
                className="w-full py-3 bg-primary text-primary-foreground rounded-xl font-bold flex items-center justify-center gap-2 hover:opacity-90 disabled:opacity-50 transition-all active:scale-[0.98]"
              >
                {status === "loading" ? (
                  <Loader2 className="w-5 h-5 animate-spin" />
                ) : (
                  <Play className="w-5 h-5" />
                )}
                Run Audio Pipeline
              </button>
            </div>
          </div>

          <div className="glass p-6 rounded-2xl border border-white/5">
            <h3 className="text-sm font-semibold mb-4 text-slate-400 uppercase tracking-wider">Features Tested</h3>
            <div className="space-y-3">
              {[
                { icon: Mic, label: "Whisper Medium STT", color: "text-blue-400" },
                { icon: Translate, label: "Ollama LLM Translation", color: "text-purple-400" },
                { icon: Volume2, label: "Coqui Voice Cloning", color: "text-emerald-400" },
                { icon: Wand2, label: "Audio Concat & Mixing", color: "text-orange-400" },
              ].map((f, i) => (
                <div key={i} className="flex items-center gap-3 text-sm text-slate-300 bg-white/5 p-2 rounded-lg border border-white/5">
                  <f.icon className={`w-4 h-4 ${f.color}`} />
                  <span>{f.label}</span>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Console / Logs Card */}
        <div className="lg:col-span-2 space-y-6">
          <div className="glass rounded-2xl border border-white/5 flex flex-col h-[600px] overflow-hidden">
            <div className="px-6 py-4 border-b border-white/5 flex items-center justify-between bg-white/5">
              <div className="flex items-center gap-2">
                <div className="flex gap-1.5">
                  <div className="w-3 h-3 rounded-full bg-red-500/50" />
                  <div className="w-3 h-3 rounded-full bg-yellow-500/50" />
                  <div className="w-3 h-3 rounded-full bg-green-500/50" />
                </div>
                <span className="text-xs font-mono text-slate-500 ml-4">audio_pipeline_terminal</span>
              </div>
              {status === "loading" && (
                <div className="text-xs text-primary animate-pulse flex items-center gap-2">
                  <Loader2 className="w-3 h-3 animate-spin" />
                  Processing...
                </div>
              )}
            </div>

            <div className="flex-1 p-6 font-mono text-sm overflow-y-auto space-y-2 bg-black/20">
              {logs.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-slate-600 gap-4">
                  <FileAudio className="w-12 h-12 opacity-20" />
                  <p>Wait for logs to appear here...</p>
                </div>
              ) : (
                logs.map((log, i) => (
                  <div key={i} className="flex gap-4 border-l-2 border-white/5 pl-4 hover:bg-white/5 transition-colors py-1">
                    <span className="text-slate-600 shrink-0">[{i}]</span>
                    <span className={log.includes("✅") ? "text-emerald-400" : log.includes("❌") ? "text-red-400" : "text-slate-300"}>
                      {log}
                    </span>
                  </div>
                ))
              )}
            </div>

            {status === "success" && resultAudio && (
              <div className="p-6 bg-emerald-500/10 border-t border-emerald-500/20 animate-in slide-in-from-bottom duration-500">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-4">
                    <div className="w-12 h-12 rounded-full bg-emerald-500 flex items-center justify-center shadow-lg shadow-emerald-500/20">
                      <Play className="w-6 h-6 text-white" />
                    </div>
                    <div>
                      <p className="font-bold text-emerald-400">Test Result Ready</p>
                      <p className="text-xs text-slate-400">Processed in {logs.length * 0.5}s</p>
                    </div>
                  </div>
                  <a 
                    href={resultAudio} 
                    target="_blank" 
                    className="px-6 py-2 bg-emerald-500 text-white rounded-lg font-bold text-sm hover:bg-emerald-600 transition-colors"
                  >
                    Download Audio
                  </a>
                </div>
              </div>
            )}
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="glass p-4 rounded-xl border border-white/5 flex items-center gap-4">
              <div className="w-10 h-10 rounded-lg bg-blue-500/10 flex items-center justify-center">
                <CheckCircle2 className="w-5 h-5 text-blue-500" />
              </div>
              <div>
                <p className="text-xs text-slate-500 uppercase font-bold tracking-widest">Model</p>
                <p className="text-sm font-semibold">Whisper v3 Medium</p>
              </div>
            </div>
            <div className="glass p-4 rounded-xl border border-white/5 flex items-center gap-4">
              <div className="w-10 h-10 rounded-lg bg-purple-500/10 flex items-center justify-center">
                <Activity className="w-5 h-5 text-purple-500" />
              </div>
              <div>
                <p className="text-xs text-slate-500 uppercase font-bold tracking-widest">Status</p>
                <p className="text-sm font-semibold">All Workers Online</p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
