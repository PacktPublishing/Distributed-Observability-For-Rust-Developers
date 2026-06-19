const PROXY_CONFIG = {
  "/api": {
    target: "http://localhost:4200",
    secure: false,
    //changeOrigin: true,
    logLevel: "debug",
    bypass: function (req, res, proxyOptions) {
      console.log(`[Proxy] ${req.method} ${req.url}`);
      // Don't bypass - let all /api requests go through
      return null;
    }
  }
};

module.exports = PROXY_CONFIG;
