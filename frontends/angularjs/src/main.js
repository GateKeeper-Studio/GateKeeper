import angular from 'angular';
import { GATEKEEPER_MODULE } from '@gatekeeper/angularjs';
import './styles.css';

const env = import.meta.env;

angular
  .module('app', [GATEKEEPER_MODULE])
  .config([
    'gatekeeperProvider',
    function (gatekeeperProvider) {
      gatekeeperProvider.configure({
        idpBaseUrl: env.VITE_GATEKEEPER_IDP_BASE_URL || 'http://localhost:8080',
        authUiBaseUrl: env.VITE_GATEKEEPER_AUTH_UI_BASE_URL || 'http://localhost:5173',
        tenantId: env.VITE_GATEKEEPER_TENANT_ID || '',
        clientId: env.VITE_GATEKEEPER_CLIENT_ID || '',
        redirectUri: env.VITE_GATEKEEPER_REDIRECT_URI || window.location.origin + '/',
        scope: env.VITE_GATEKEEPER_SCOPE || 'openid profile email'
      });
    }
  ])
  .controller('AppController', [
    '$scope',
    'gatekeeper',
    function ($scope, gatekeeper) {
      const vm = this;
      vm.gatekeeper = gatekeeper;
      vm.output = 'No userinfo loaded.';

      vm.formatExpiry = function (seconds) {
        return new Date(seconds * 1000).toLocaleString();
      };

      // The SDK uses native promises; wrap view updates in a digest.
      vm.loadUserInfo = function () {
        gatekeeper.fetchUserInfo().then(
          function (claims) {
            $scope.$applyAsync(function () {
              vm.output = JSON.stringify(claims, null, 2);
            });
          },
          function (err) {
            $scope.$applyAsync(function () {
              vm.output = err && err.message ? err.message : 'Request failed';
            });
          }
        );
      };

      vm.showToken = function () {
        gatekeeper.getAccessToken().then(function (token) {
          $scope.$applyAsync(function () {
            vm.output = token
              ? 'access_token (truncated): ' + token.slice(0, 32) + '…'
              : 'No access token.';
          });
        });
      };
    }
  ]);

angular.element(function () {
  angular.bootstrap(document.body, ['app'], { strictDi: true });
});
