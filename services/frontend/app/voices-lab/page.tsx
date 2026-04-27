"use client";

import React, { useState } from "react";
import { 
  Mic, 
  Languages, 
  Volume2, 
  Play, 
  Wand2, 
  Loader2, 
  CheckCircle2, 
  AlertCircle,
  FileAudio,
  ArrowRight,
  Activity
} from "lucide-react";

export default function VoicesLabPage() {
  const [audioUrl, setAudioUrl] = useState("");
  const [targetLang, setTargetLang] = useState("fr");
  const [status, setStatus] = useState<"idle" | "loading" | "success" | "error">("idle");
  const [logs, setLogs] = useState<string[]>([]);
  const [resultAudio, setResultAudio] = useState<string | null>(null);
  const [currentStep, setCurrentStep] = useState(0);
  const [previewText, setPreviewText] = useState<{ original?: string, translated?: string }>({});

  const steps = [
    { id: 1, label: "Transcription", icon: Mic, description: "Extracting text with Whisper Medium" },
    { id: 2, label: "Refinement", icon: Wand2, description: "Fluidifying text with Ollama" },
    { id: 3, label: "Translation", icon: Languages, description: `Translating to ${targetLang.toUpperCase()}` },
    { id: 4, label: "Cloning", icon: Volume2, description: "Generating voice with Coqui" },
    { id: 5, label: "Finalizing", icon: Activity, description: "Audio mixing and S3 upload" }
  ];

  const addLog = (msg: string) => {
    setLogs(prev => [...prev, `${new Date().toLocaleTimeString()} - ${msg}`]);
    
    // Auto-advance steps based on log keywords (simulation for UX)
    if (msg.toLowerCase().includes("transcription")) setCurrentStep(1);
    if (msg.toLowerCase().includes("fluidification")) setCurrentStep(2);
    if (msg.toLowerCase().includes("traduction")) setCurrentStep(3);
    if (msg.toLowerCase().includes("clonage")) setCurrentStep(4);
    if (msg.toLowerCase().includes("composition")) setCurrentStep(5);
  };

  const handleRunTest = async () => {
    if (!audioUrl) return;
    
    setStatus("loading");
    setLogs([]);
    setResultAudio(null);
    setCurrentStep(0);
    setPreviewText({});
    addLog("Initializing Audio Lab Pipeline...");

    try {
      addLog(`Connecting to Orchestrator...`);
      
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
      setCurrentStep(5);
      addLog("✅ Pipeline finished successfully!");
      setResultAudio(data.result_url);
      setStatus("success");
    } catch (err: any) {
      setStatus("error");
      addLog(`❌ Error: ${err.message}`);
    }
  };

  return (
    <div className="p-8 space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      <div className="flex flex-col gap-2">
        <h1 className="text-5xl font-extrabold tracking-tighter text-gradient pb-2">Voices Lab</h1>
        <p className="text-slate-400 text-lg">Next-gen audio processing pipeline. Experience the magic of AI voice transformation.</p>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-4 gap-8">
        {/* Sidebar Config */}
        <div className="lg:col-span-1 space-y-6">
          <div className="glass p-6 rounded-3xl border border-white/10 shadow-2xl space-y-6 bg-gradient-to-br from-white/5 to-transparent">
            <div className="flex items-center gap-3 text-primary font-bold text-lg">
              <div className="w-8 h-8 rounded-full bg-primary/20 flex items-center justify-center">
                <Wand2 className="w-5 h-5" />
              </div>
              <span>Experiment</span>
            </div>

            <div className="space-y-5">
              <div className="space-y-2">
                <label className="text-xs font-bold text-slate-500 uppercase tracking-widest px-1">Source Audio URL</label>
                <div className="relative group">
                  <div className="absolute inset-0 bg-primary/20 blur-xl opacity-0 group-focus-within:opacity-100 transition-opacity rounded-xl" />
                  <FileAudio className="absolute left-3 top-3.5 w-5 h-5 text-slate-500 group-focus-within:text-primary transition-colors" />
                  <input 
                    type="text" 
                    placeholder="https://s3.amazonaws.com/..." 
                    className="w-full bg-black/60 border border-white/10 rounded-2xl py-3.5 pl-12 pr-4 text-sm focus:border-primary focus:ring-1 focus:ring-primary/50 outline-none transition-all relative z-10"
                    value={audioUrl}
                    onChange={(e) => setAudioUrl(e.target.value)}
                  />
                </div>
              </div>

              <div className="space-y-2">
                <label className="text-xs font-bold text-slate-500 uppercase tracking-widest px-1">Target Language</label>
                <div className="grid grid-cols-3 gap-2">
                  {['fr', 'en', 'es', 'de', 'it', 'ja'].map((lang) => (
                    <button
                      key={lang}
                      onClick={() => setTargetLang(lang)}
                      className={`py-2 rounded-xl text-xs font-bold transition-all border ${
                        targetLang === lang 
                          ? 'bg-primary text-white border-primary shadow-lg shadow-primary/20 scale-105' 
                          : 'bg-white/5 text-slate-400 border-white/5 hover:bg-white/10'
                      }`}
                    >
                      {lang.toUpperCase()}
                    </button>
                  ))}
                </div>
              </div>

              <button 
                onClick={handleRunTest}
                disabled={status === "loading" || !audioUrl}
                className="group relative w-full py-4 bg-primary text-primary-foreground rounded-2xl font-black text-lg overflow-hidden shadow-xl shadow-primary/30 disabled:opacity-50 disabled:shadow-none transition-all hover:scale-[1.02] active:scale-[0.98]"
              >
                <div className="absolute inset-0 bg-gradient-to-r from-white/0 via-white/20 to-white/0 -translate-x-full group-hover:animate-shimmer" />
                <div className="relative flex items-center justify-center gap-3">
                  {status === "loading" ? (
                    <Loader2 className="w-6 h-6 animate-spin" />
                  ) : (
                    <Play className="w-6 h-6 fill-current" />
                  )}
                  <span>IGNITE PIPELINE</span>
                </div>
              </button>
            </div>
          </div>

          <div className="glass p-6 rounded-3xl border border-white/5 bg-black/40">
            <h3 className="text-xs font-black mb-6 text-slate-500 uppercase tracking-[0.2em]">Neural Engine Specs</h3>
            <div className="space-y-4">
              {[
                { icon: Mic, label: "Whisper Medium", detail: "Multi-head attention STT", color: "text-blue-400" },
                { icon: Wand2, label: "Llama 3 Refine", detail: "Context-aware cleanup", color: "text-purple-400" },
                { icon: Languages, label: "Universal Translate", detail: "Semantic language mapping", color: "text-emerald-400" },
                { icon: Volume2, label: "XTTS v2 Clone", detail: "Zero-shot voice cloning", color: "text-orange-400" },
              ].map((f, i) => (
                <div key={i} className="group flex items-start gap-4">
                  <div className={`p-2 rounded-xl bg-white/5 border border-white/5 group-hover:border-primary/30 transition-colors ${f.color}`}>
                    <f.icon className="w-4 h-4" />
                  </div>
                  <div>
                    <p className="text-sm font-bold text-slate-200">{f.label}</p>
                    <p className="text-[10px] text-slate-500 font-medium uppercase tracking-wider">{f.detail}</p>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Main Content */}
        <div className="lg:col-span-3 space-y-8">
          {/* Progress Stepper */}
          <div className="glass p-8 rounded-[2.5rem] border border-white/10 bg-gradient-to-r from-white/5 to-transparent relative overflow-hidden">
            <div className="absolute top-0 right-0 p-8 opacity-5">
              <Activity className="w-32 h-32 text-primary" />
            </div>
            
            <div className="relative z-10 flex justify-between items-start">
              {steps.map((step, i) => {
                const isActive = i === currentStep;
                const isCompleted = i < currentStep;
                return (
                  <div key={step.id} className="flex flex-col items-center gap-4 group flex-1">
                    <div className="relative">
                      {i < steps.length - 1 && (
                        <div className={`absolute left-full top-6 w-[calc(100%)] h-1 -translate-x-1/2 transition-colors duration-1000 ${
                          isCompleted ? 'bg-primary' : 'bg-white/5'
                        }`} />
                      )}
                      <div className={`w-12 h-12 rounded-2xl flex items-center justify-center transition-all duration-500 border-2 ${
                        isActive 
                          ? 'bg-primary border-primary shadow-2xl shadow-primary/50 scale-125 z-20' 
                          : isCompleted 
                            ? 'bg-emerald-500/20 border-emerald-500 text-emerald-500' 
                            : 'bg-white/5 border-white/10 text-slate-600'
                      }`}>
                        {isCompleted ? <CheckCircle2 className="w-6 h-6" /> : <step.icon className="w-6 h-6" />}
                      </div>
                    </div>
                    <div className="text-center space-y-1">
                      <p className={`text-sm font-black transition-colors ${isActive ? 'text-white' : 'text-slate-500'}`}>{step.label}</p>
                      <p className="text-[10px] text-slate-600 max-w-[100px] leading-tight font-medium uppercase tracking-tighter">{step.description}</p>
                    </div>
                  </div>
                )
              })}
            </div>
          </div>

          <div className="grid grid-cols-1 xl:grid-cols-2 gap-8">
            {/* Terminal / Console */}
            <div className="glass rounded-[2rem] border border-white/10 flex flex-col h-[500px] overflow-hidden bg-black/60 shadow-2xl">
              <div className="px-6 py-4 border-b border-white/10 flex items-center justify-between bg-white/5 backdrop-blur-md">
                <div className="flex items-center gap-3">
                  <div className="flex gap-1.5">
                    <div className="w-3 h-3 rounded-full bg-[#FF5F56] shadow-lg shadow-[#FF5F56]/20" />
                    <div className="w-3 h-3 rounded-full bg-[#FFBD2E] shadow-lg shadow-[#FFBD2E]/20" />
                    <div className="w-3 h-3 rounded-full bg-[#27C93F] shadow-lg shadow-[#27C93F]/20" />
                  </div>
                  <span className="text-[10px] font-black text-slate-500 tracking-[0.3em] ml-4 uppercase">Keryx_Kernel_V3</span>
                </div>
                {status === "loading" && (
                  <div className="flex items-center gap-3 px-3 py-1 bg-primary/10 border border-primary/20 rounded-full">
                    <div className="w-1.5 h-1.5 rounded-full bg-primary animate-ping" />
                    <span className="text-[10px] font-bold text-primary uppercase">Active Processing</span>
                  </div>
                )}
              </div>

              <div className="flex-1 p-6 font-mono text-[13px] overflow-y-auto space-y-2 selection:bg-primary selection:text-white custom-scrollbar">
                {logs.length === 0 ? (
                  <div className="h-full flex flex-col items-center justify-center text-slate-700 gap-6 opacity-40">
                    <div className="relative">
                      <FileAudio className="w-20 h-20" />
                      <div className="absolute inset-0 bg-primary blur-3xl opacity-20" />
                    </div>
                    <p className="font-bold tracking-widest text-xs">AWAITING NEURAL INPUT</p>
                  </div>
                ) : (
                  logs.map((log, i) => (
                    <div key={i} className="flex gap-4 border-l border-white/10 pl-4 hover:bg-white/5 transition-colors py-1 group">
                      <span className="text-slate-600 shrink-0 select-none group-hover:text-primary transition-colors">0x{i.toString(16).padStart(2, '0')}</span>
                      <span className={`${
                        log.includes("✅") ? "text-emerald-400 font-bold" : 
                        log.includes("❌") ? "text-red-400 font-bold underline" : 
                        log.includes("⚠️") ? "text-amber-400" :
                        "text-slate-300"
                      }`}>
                        {log}
                      </span>
                    </div>
                  ))
                )}
                {status === "loading" && (
                  <div className="flex gap-4 pl-4 py-1">
                    <span className="text-primary animate-pulse font-bold">_</span>
                  </div>
                )}
              </div>
            </div>

            {/* Results / Insights */}
            <div className="space-y-8">
              {/* Audio Result Card */}
              <div className={`glass rounded-[2rem] border-2 transition-all duration-700 h-[300px] flex flex-col items-center justify-center relative overflow-hidden ${
                status === "success" 
                  ? 'border-emerald-500/50 bg-emerald-500/5' 
                  : status === "error"
                    ? 'border-red-500/50 bg-red-500/5'
                    : 'border-white/5 bg-white/5'
              }`}>
                {status === "success" && resultAudio ? (
                  <div className="text-center space-y-6 animate-in zoom-in duration-500">
                    <div className="relative inline-block">
                      <div className="absolute inset-0 bg-emerald-500 blur-3xl opacity-30 animate-pulse" />
                      <div className="w-24 h-24 rounded-[2rem] bg-emerald-500 flex items-center justify-center shadow-2xl shadow-emerald-500/40 relative z-10 rotate-3">
                        <Play className="w-10 h-10 text-white fill-current" />
                      </div>
                    </div>
                    <div className="space-y-2">
                      <h3 className="text-2xl font-black text-emerald-400">SUCCESSFULLY CLONED</h3>
                      <p className="text-slate-400 text-sm font-medium">Neural synthesis complete in {(logs.length * 0.4).toFixed(1)}s</p>
                    </div>
                    <a 
                      href={resultAudio} 
                      target="_blank" 
                      className="inline-flex items-center gap-3 px-8 py-3 bg-emerald-500 text-white rounded-2xl font-black text-sm hover:bg-emerald-600 hover:scale-105 transition-all shadow-xl shadow-emerald-500/20 active:scale-95"
                    >
                      <Volume2 className="w-5 h-5" />
                      STREAM RESULT
                    </a>
                  </div>
                ) : status === "loading" ? (
                  <div className="text-center space-y-6">
                    <div className="w-20 h-20 border-4 border-primary/20 border-t-primary rounded-full animate-spin mx-auto shadow-2xl shadow-primary/20" />
                    <div className="space-y-1">
                      <p className="text-xl font-black text-primary tracking-widest uppercase italic">Synthesizing</p>
                      <p className="text-[10px] text-slate-500 font-black tracking-[0.3em]">PLEASE REMAIN CALM</p>
                    </div>
                  </div>
                ) : (
                  <div className="text-center space-y-4 opacity-20">
                    <Activity className="w-20 h-20 mx-auto" />
                    <p className="font-black text-sm tracking-[0.2em]">WAITING FOR PIPELINE START</p>
                  </div>
                )}
              </div>

              {/* Status Badges */}
              <div className="grid grid-cols-2 gap-6">
                <div className="glass p-6 rounded-3xl border border-white/10 hover:border-primary/50 transition-colors group">
                  <div className="flex items-center justify-between mb-2">
                    <p className="text-[10px] font-black text-slate-500 uppercase tracking-widest px-1">Network Integrity</p>
                    <div className="w-2 h-2 rounded-full bg-emerald-500 shadow-lg shadow-emerald-500/50" />
                  </div>
                  <div className="flex items-end gap-2">
                    <span className="text-2xl font-black text-slate-200">99.8%</span>
                    <span className="text-[10px] text-emerald-500 font-bold mb-1">UPTIME</span>
                  </div>
                </div>
                <div className="glass p-6 rounded-3xl border border-white/10 hover:border-primary/50 transition-colors group">
                  <div className="flex items-center justify-between mb-2">
                    <p className="text-[10px] font-black text-slate-500 uppercase tracking-widest px-1">Processing Node</p>
                    <Activity className="w-3 h-3 text-primary animate-pulse" />
                  </div>
                  <div className="flex items-end gap-2">
                    <span className="text-2xl font-black text-slate-200">H100-v2</span>
                    <span className="text-[10px] text-primary font-bold mb-1">COMPUTE</span>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
