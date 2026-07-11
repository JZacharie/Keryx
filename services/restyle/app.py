import os
import gradio as gr
from pipeline import execution_complete

def process_image(input_img, style_prompt, gemini_api_key):
    if input_img is None:
        return None, "(aucune image fournie)"
    try:
        output_img, new_text = execution_complete(input_img, style_prompt, gemini_api_key)
        return output_img, new_text
    except Exception as e:
        return input_img, f"Erreur : {e}"

with gr.Blocks(theme=gr.themes.Soft(), title="Restyle - Restylisation de Slides") as demo:
    gr.Markdown(
        """
        # 🎨 Restyle
        Transformez vos slides avec OCR + IA — texte reformulé, visuel restylisé.
        """
    )

    with gr.Row():
        with gr.Column(scale=1):
            input_image = gr.Image(
                type="pil",
                label="Slide originale",
                height=400,
            )
            style_prompt = gr.Textbox(
                label="Consigne de style",
                placeholder="Ex: Rends le ton plus fun et dynamique...",
                lines=2,
            )
            api_key_input = gr.Textbox(
                label="Clé API Gemini",
                type="password",
                placeholder="AIza...",
                value=os.getenv("GEMINI_API_KEY", ""),
            )
            submit_btn = gr.Button(
                "🚀 Transformer la slide",
                variant="primary",
                size="lg",
            )

        with gr.Column(scale=1):
            output_image = gr.Image(
                type="pil",
                label="Slide restylisée",
                height=400,
            )
            output_text = gr.Textbox(
                label="Texte reformulé",
                lines=6,
                interactive=True,
            )

    submit_btn.click(
        fn=process_image,
        inputs=[input_image, style_prompt, api_key_input],
        outputs=[output_image, output_text],
    )

    gr.Markdown(
        """
        ---
        ### 📋 Comment ça marche
        1. **Glissez** votre slide (PNG/JPG)
        2. **Ajoutez** une consigne de style (optionnel)
        3. **Entrez** votre clé API Gemini
        4. **Cliquez** sur "Transformer"
        """
    )

if __name__ == "__main__":
    port = int(os.getenv("PORT", "7860"))
    demo.launch(server_name="0.0.0.0", server_port=port)
