const supertest = require('supertest');
const expect = require('chai').expect;
const mocha = require('mocha');
const tv4 = require('tv4');
const fs = require('fs');
const pm = require('../../pm');const debug=require('debug');var log = debug('bn-api');

const baseUrl = supertest(pm.environment.get('server'));

const apiEndPoint = '/organizations/{{last_org_id}}/invites';


var response;
var responseBody;


const post = async function (request_body) {
    return baseUrl
        .post(pm.substitute(apiEndPoint))
        .set('Accept', 'application/json')
        .set('Content-Type', 'application/json')
        .set('Authorization', pm.substitute('Bearer {{token}}'))

        .send(pm.substitute(request_body));
};

let requestBody = `{
"user_email": "{{last_org_member_email}}",
 "roles" : ["OrgMember"]
}`;


describe('OrgAdmin - Invite OrgMember', function () {
    before(async function () {
        pm.environment.set("last_org_member_email", "orgmember" + Math.floor(Math.random() * 10000) + "@test.com");

        response = await post(requestBody);
        log(response.request.header);
        log(response.request.url);
        log(response.request._data);
        log(response.request.method);
        responseBody = JSON.stringify(response.body);
        //log(pm);
        log(response.status);
        log(responseBody);
    });

    after(async function () {
        // add after methods
        let r = JSON.parse(responseBody);

        pm.environment.set("last_invite_token", r.security_token);

        pm.environment.set("last_invite_id", r.id);

    });

    it("should be 201", function () {
        expect(response.status).to.equal(201);
    })


});


