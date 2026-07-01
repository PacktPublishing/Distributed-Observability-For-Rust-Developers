<a href="https://www.packtpub.com/en-us/unlock"><img src="https://drive.google.com/uc?export=view&id=1lQCTQQ8iV5pGuPA1n5wuds-3pwJi0OD_"></a>
<h1 align="center">
Mastering Distributed Observability in Rust, First Edition</h1>
<p align="center">This is the code repository for <a href ="https://www.packtpub.com/en-us/product/mastering-distributed-observability-in-rust-first-edition/9781806671793"> Mastering Distributed Observability in Rust, First Edition</a>, published by Packt.
</p>

<h2 align="center">
Implement OpenTelemetry in a real-world, multi-container e-commerce architecture
</h2>
<p align="center">
Manjunath Gangappa, Rajkumar Rangaraj</p>

<p align="center">
    &#8287;&#8287;&#8287;&#8287;&#8287;
  <a href="https://packt.link/free-ebook/9781806671793"><img width="32px" alt="Free PDF" title="Free PDF" src="https://cdn-icons-png.flaticon.com/512/4726/4726010.png"/></a>
 &#8287;&#8287;&#8287;&#8287;&#8287;
  <a href="https://packt.link/gbp/9781806671793"><img width="32px" alt="Graphic Bundle" title="Graphic Bundle" src="https://cdn-icons-png.flaticon.com/512/2659/2659360.png"/></a>
  &#8287;&#8287;&#8287;&#8287;&#8287;
   <a href="https://www.amazon.in/Mastering-Distributed-Observability-Rust-commerce-ebook/dp/B0GXFW35K2"><img width="32px" alt="Amazon" title="Get your copy" src="https://cdn-icons-png.flaticon.com/512/15466/15466027.png"/></a>
  &#8287;&#8287;&#8287;&#8287;&#8287;
</p>
<details open> 
  <summary><h2>About the book</summary>
<a href="https://www.packtpub.com/en-us/product/mastering-distributed-observability-in-rust-first-edition/9781806671793">
<img src="https://content.packt.com/B36968/cover_image_small.jpg" alt="Mastering Distributed Observability in Rust, First Edition" height="256px" align="right">
</a>

Gain the skills to build, monitor, and debug distributed systems in Rust with this hands-on guide to observability using OpenTelemetry. As Rust adoption grows in backend services, developers face fragmented documentation and limited tooling for telemetry. This book fills that gap by presenting a unified, end-to-end solution to implement distributed observability in modern Rust systems.

You’ll explore the foundations of observability and Rust’s ownership model before learning how to collect, export, and correlate logs, metrics, and traces. Discover how to instrument applications using OpenTelemetry crates and bridge them with the tracing ecosystem. Learn to deploy the OpenTelemetry Collector, integrate with Prometheus, Grafana, and Jaeger, and tackle challenges like sampling, context propagation, and async tracing.

Written by two seasoned engineers with over 35 years of combined experience in large-scale systems and open-source observability leadership, this book balances theory with real-world implementations. From debugging async bottlenecks to configuring cost-effective telemetry pipelines, you’ll finish with the confidence to operate reliable, observable Rust systems at scale.</details>
<details open> 
  <summary><h2>Key Learnings</summary>
<ul>

<li>Understand the three pillars of observability in Rust</li>

<li>Apply tracing and logging using the Tokio and OpenTelemetry crates</li>

<li>Export telemetry to backends like Prometheus, Jaeger, and Grafana</li>

<li>Use the OpenTelemetry Collector to manage telemetry pipelines</li>

<li>Correlate metrics, logs, and traces for faster debugging</li>

<li>Implement structured logging, redaction, and context propagation</li>

<li>Optimize telemetry costs with sampling strategies</li>

<li>Build and deploy an observability-first Rust microservice</li>

</ul>

  </details>

<details open> 
  <summary><h2>Chapters</summary>
     <img src="https://cliply.co/wp-content/uploads/2020/02/372002150_DOCUMENTS_400px.gif" alt="Unity Cookbook, Fifth Edition" height="556px" align="right">
<ol>

  <li>Introduction to Observability</li>

  <li>Reading Rust Memory from Traces</li>

  <li>The OpenTel E-Commerce System</li>

  <li>Instrumenting the Request Journey</li>

  <li>Data Layer Instrumentation</li>

  <li>Metrics That Matter (RED + USE + KPIs)</li>

  <li>Log Strategy Without Log Hell</li>

  <li>Business Intelligence Views From Telemetry</li>

  <li>Finding Bottlenecks with Flamegraphs</li>

  <li>Async Bottlenecks and Runtime Saturation</li>

  <li>Database Bottlenecks</li>

  <li>Debugging Production Incidents</li>

  <li>Detecting Attacks in Traces</li>

  <li>Observability for AI-Augmented Services</li>

</ol>

</details>


<details open> 
  <summary><h2>Requirements for this book</summary>
The sample project, OpenTel E-Commerce, runs entirely on your local machine through Docker
Compose. To follow along, you will need the following installed:
    <ul>
<li>Rust 1.8 or later, installed through rustup, with Cargo</li>
<li>Docker and Docker Compose, used to run PostgreSQL and the telemetry backends</li>
<li>Git, for cloning the sample repository</li>
<li>A code editor with Rust support, such as VS Code with rust-analyzer, or RustRover</li>
<li>A terminal that supports Unix-style commands, which means WSL2 is recommended on Windows</li>
    </ul>
  </details>
    


<details> 
  <summary><h2>Get to know Authors</h2></summary>

_Manjunath Gangappa_ Manjunath Gangappa is a Director of Software Engineering at Mastercard R&D, with over 19 years of experience in enterprise software. He brings more than six years of hands-on Rust development along with extensive experience in the Java ecosystem, UI development (JavaScript), database design, and large-scale system architecture. At Mastercard, he drives the development of cross-border payment platforms that incorporate blockchain technology, blending deep technical expertise with practical leadership. Having implemented observability in mission-critical financial systems, he offers firsthand insight into why telemetry spanning tracing, metrics, and logging is indispensable for ensuring reliability, security, and trust in modern FinTech.

_Rajkumar Rangaraj_ Rajkumar Rangaraj is a Principal Software Engineer at Microsoft with 19 years of experience in designing and building large-scale distributed systems and developer platforms. As a long-standing maintainer and contributor in the OpenTelemetry project, he plays an active role in shaping the future of observability across the industry. His work spans defining cross-language instrumentation strategies, architecting telemetry pipelines, and driving the adoption of open standards at scale. Rajkumar has authored migration strategies and technical specifications that help organizations transition from legacy monitoring approaches to modern, standards-based solutions. His unique perspective—bridging open-source leadership with enterprise-scale experience—positions him as both a practitioner and an architect of the next generation of observability.



</details>
<details> 
  <summary><h2>Other Related Books</h2></summary>
<ul>

  <li><a href="https://www.packtpub.com/en-us/product/design-patterns-and-best-practices-in-rust-first-edition/9781836209478">Design Patterns and Best Practices in Rust, First Edition</a></li>

  <li><a href="https://www.packtpub.com/en-us/product/the-rust-programming-handbook-first-edition/9781836208877">The Rust Programming Handbook, First Edition</a></li>
 
</ul>

</details>
