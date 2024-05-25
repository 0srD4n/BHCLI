import os
import google.generativeai as genai

genai.configure(api_key=os.environ["AIzaSyCCV8za9Q-a8mnWqQ0-OWBMkgfPTfmW4xg"])

generation_config = {
  "temperature": 1,
  "top_p": 0.95,
  "top_k": 64,
  "max_output_tokens": 8192,
  "response_mime_type": "text/plain",
}
safety_settings = [
  {
    "category": "HARM_CATEGORY_HARASSMENT",
    "threshold": "BLOCK_MEDIUM_AND_ABOVE",
  },
  {
    "category": "HARM_CATEGORY_HATE_SPEECH",
    "threshold": "BLOCK_MEDIUM_AND_ABOVE",
  },
  {
    "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
    "threshold": "BLOCK_MEDIUM_AND_ABOVE",
  },
  {
    "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
    "threshold": "BLOCK_MEDIUM_AND_ABOVE",
  },
]

model = genai.GenerativeModel(
  model_name="gemini-1.5-pro-latest",
  safety_settings=safety_settings,
  generation_config=generation_config,
  system_instruction="reaspon in indonesia",
)

chat_session = model.start_chat(
  history=[
    {
      "role": "user",
      "parts": [
        "cara configurasi ai studio dengan rust \n",
      ],
    },
    {
      "role": "model",
      "parts": [
        "Sayangnya, Google AI Platform Notebooks (sebelumnya dikenal sebagai Google AI Studio) tidak menyediakan dukungan langsung untuk menjalankan Rust. Google AI Platform Notebooks lebih difokuskan pada Python dan kerangka kerja Machine Learning berbasis Python seperti TensorFlow, PyTorch, dan Scikit-learn.\n\nNamun, ada beberapa cara untuk memanfaatkan Rust pada Google AI Platform Notebooks secara tidak langsung:\n\n1. **Kompilasi Rust ke WebAssembly (WASM)**:\n   * Anda dapat mengkompilasi kode Rust Anda ke WebAssembly dan menjalankannya di browser web melalui JavaScript. \n   * Ini memungkinkan Anda untuk menggunakan Rust untuk logika komputasi performa tinggi dalam aplikasi web yang dihosting di Google AI Platform Notebooks.\n   * Anda membutuhkan perpustakaan JavaScript yang sesuai untuk berinteraksi dengan kode WASM.\n\n2. **Panggilan ke kode Rust melalui Python**:\n   * Anda dapat membuat executable Rust dan memanggilnya dari kode Python menggunakan library `subprocess` di Python.\n   * Cara ini memungkinkan Anda untuk memanfaatkan Rust untuk tugas-tugas spesifik yang membutuhkan performa tinggi, sementara tetap menggunakan Python sebagai bahasa utama Anda di Google AI Platform Notebooks.\n\n3. **Docker Container**:\n   * Anda dapat membangun Docker container yang berisi lingkungan Rust dan menggunakan Google AI Platform Notebooks untuk menjalankan container tersebut. \n   * Ini memberi Anda fleksibilitas penuh dalam menggunakan Rust, tetapi membutuhkan pengetahuan tentang Docker dan pengaturan container.\n\nPenting untuk dicatat bahwa metode-metode ini membutuhkan pengetahuan tentang Rust, WebAssembly, Python, dan Docker. \n\nBerikut adalah beberapa sumber daya yang mungkin berguna:\n\n* **Rust**: https://www.rust-lang.org/\n* **WebAssembly**: https://webassembly.org/\n* **Python**: https://www.python.org/\n* **Docker**: https://www.docker.com/\n\nMeskipun Google AI Platform Notebooks tidak mendukung Rust secara langsung, Anda tetap dapat memanfaatkan Rust dengan cara yang kreatif untuk memenuhi kebutuhan Anda. \n",
      ],
    },
    {
      "role": "user",
      "parts": [
        "Panggilan ke kode Rust melalui Python:\n\n    Anda dapat membuat executable Rust dan memanggilnya dari kode Python menggunakan library subprocess di Python.\n\n    Cara ini memungkinkan Anda untuk memanfaatkan Rust untuk tugas-tugas spesifik yang membutuhkan performa tinggi, sementara tetap menggunakan Python sebagai bahasa utama Anda di Google AI Platform Notebooks caranya gimana \n",
      ],
    },
    {
      "role": "model",
      "parts": [

