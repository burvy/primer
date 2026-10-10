// Google's "Sign in with Google" button, for the parent's sign-in page.
//
//   primerSignIn.render(clientId, element, onCredential)
//
// Loads Google's own script the first time it is needed (only on the sign-in
// page, and only when the server has a Google client id), draws Google's
// button inside `element`, and calls `onCredential(idToken)` when the parent
// has signed in. The page sends that token to the server, which asks Google
// whether it is genuine and made for this app.

(function () {
  "use strict";

  function load(then) {
    if (window.google && window.google.accounts) {
      then();
      return;
    }
    const script = document.createElement("script");
    script.src = "https://accounts.google.com/gsi/client";
    script.async = true;
    script.onload = then;
    document.head.appendChild(script);
  }

  function render(clientId, element, onCredential) {
    load(() => {
      window.google.accounts.id.initialize({
        client_id: clientId,
        callback: (response) => onCredential(response.credential),
      });
      window.google.accounts.id.renderButton(element, { theme: "outline", size: "large" });
    });
  }

  window.primerSignIn = { render };
})();
